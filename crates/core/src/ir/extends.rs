//! The `extends` module expands `$extends` in the lowered tree, so later passes never have to know it existed.
//!
//! `"$extends": "{base}"` means: start with everything in `base`, then apply the local group on top. The spec calls
//! this a deep merge: the same path is overridden by the local definition (a token is replaced whole), and
//! anything new is added. After expansion an inherited token is a real token at its new path, so
//! `{derived.x}` and `#/derived/x` resolve like any other token.
//!
//! Expansion copies props with fresh [`PropId`]s. That is the reason lowering has its own ID space: `base.x` and
//! `derived.x` have the same raw node but are different nodes in the reference graph.
//!
//! The order matters. If `c` extends `b` and `b` extends `a`, `b` must be expanded before `c` copies it, and a
//! target path may even go *through* a group that is itself extended (`{derived.x}` where `derived` extends
//! `base`). So the pass repeats: expand every group whose target resolves and whose target subtree has no
//! unexpanded `$extends` left. What remains afterwards is either unresolvable or stuck in a cycle.

use indexmap::{IndexMap, IndexSet};
use petgraph::{algo::tarjan_scc, graph::DiGraph};

use crate::{
    ast::reference::{JsonPointer, JsonReference},
    diagnostics::{
        code::DiagnosticCode,
        diagnostic::{Diagnostic, DiagnosticSink},
    },
    ir::{
        arena::Arena,
        lower::{CommonProps, Lowered, PropId, PropKind, TokenBody, ValueData},
        node::DtcgValueSyntax,
        refs::{is_alias_like, parse_alias},
    },
};

#[derive(Debug, Clone)]
enum Target {
    /// `{group.name}`, followed through group members only.
    Alias(Vec<String>),
    /// `{"$ref": "#/group/name"}`.
    Pointer(JsonPointer),
}

/// A group that still has an unexpanded `$extends`.
struct Pending {
    extends_prop: PropId,
    target: Target,
}

pub fn expand_extends(arena: &Arena, lowered: &mut Lowered, diags: &mut impl DiagnosticSink) {
    let mut pending = collect_pending(arena, lowered, diags);

    loop {
        let mut progressed = false;

        for group in pending.keys().copied().collect::<Vec<_>>() {
            let Some(target_id) = resolve(lowered, &pending[&group].target) else {
                continue;
            };

            if !matches!(lowered.get(target_id).kind, PropKind::Group(_)) {
                report_invalid_target(arena, lowered, &pending[&group], target_id, diags);
                pending.shift_remove(&group);
                progressed = true;
            } else if !subtree_has_pending(lowered, target_id, &pending) {
                merge_into(lowered, group, target_id);
                pending.shift_remove(&group);
                progressed = true;
            }
        }

        if !progressed {
            break;
        }
    }

    report_leftovers(arena, lowered, &pending, diags);
}

fn collect_pending(
    arena: &Arena,
    lowered: &Lowered,
    diags: &mut impl DiagnosticSink,
) -> IndexMap<PropId, Pending> {
    let mut pending = IndexMap::new();

    for i in 0..lowered.len() as u32 {
        let id = PropId(i);
        let PropKind::Group(group) = &lowered.get(id).kind else {
            continue;
        };
        let Some(extends_prop) = group.extends else {
            continue;
        };
        if let Some(target) = parse_target(arena, lowered, extends_prop, diags) {
            pending.insert(id, Pending { extends_prop, target });
        }
    }

    pending
}

fn parse_target(
    arena: &Arena,
    lowered: &Lowered,
    prop: PropId,
    diags: &mut impl DiagnosticSink,
) -> Option<Target> {
    let raw = arena.get(lowered.get(prop).origin);

    let error = |diags: &mut dyn DiagnosticSink, message: String, label: &str, help: Option<String>| {
        let mut diag = Diagnostic::new(DiagnosticCode::InvalidReference)
            .with_message(message)
            .primary(raw.span.clone(), label);
        if let Some(help) = help {
            diag = diag.help(help);
        }
        diags.emit(diag);
    };

    match &raw.value {
        DtcgValueSyntax::String(text) => match parse_alias(text) {
            Some(segments) => Some(Target::Alias(segments)),
            None => {
                let message = if is_alias_like(text) {
                    format!("`{text}` is not a valid alias")
                } else {
                    format!("`$extends` must be an alias such as `{{group}}`, found `{text}`")
                };
                error(diags, message, "expected `{group.name}`", None);
                None
            }
        },
        DtcgValueSyntax::Object(members) => {
            // The last duplicate key wins, as everywhere else.
            let reference = members.iter().rev().find(|m| m.key.value == "$ref")?;
            let ref_raw = arena.get(reference.value);
            let DtcgValueSyntax::String(text) = &ref_raw.value else {
                error(diags, "`$ref` must be a string".into(), "expected a JSON pointer string", None);
                return None;
            };

            match JsonReference::from_str(text) {
                Ok(reference) if reference.document().is_none() => {
                    Some(Target::Pointer(reference.fragment().clone()))
                }
                Ok(reference) => {
                    let path = reference.document().and_then(|d| d.path()).unwrap_or_default();
                    error(
                        diags,
                        format!("`$extends` references to other files (`{path}`) are not supported yet"),
                        "only `#/...` references are resolved",
                        None,
                    );
                    None
                }
                Err(err) => {
                    error(diags, format!("`{text}` is not a valid reference"), &format!("{err:?}"), err.hint());
                    None
                }
            }
        }
        // `Invalid` was reported by the parser, and other shapes by lowering.
        _ => None,
    }
}

fn resolve(lowered: &Lowered, target: &Target) -> Option<PropId> {
    match target {
        Target::Alias(segments) => lowered.member_path(segments),
        Target::Pointer(pointer) => lowered.resolve_pointer(pointer),
    }
}

fn show(target: &Target) -> String {
    match target {
        Target::Alias(segments) => format!("{{{}}}", segments.join(".")),
        Target::Pointer(pointer) => format!("#{}", pointer.to_pointer_string()),
    }
}

fn report_invalid_target(
    arena: &Arena,
    lowered: &Lowered,
    pending: &Pending,
    target_id: PropId,
    diags: &mut impl DiagnosticSink,
) {
    let span = arena.get(lowered.get(pending.extends_prop).origin).span.clone();
    let kind = match lowered.get(target_id).kind {
        PropKind::Token(_) => "a token",
        _ => "not a group",
    };
    diags.emit(
        Diagnostic::new(DiagnosticCode::InvalidExtendsTarget)
            .with_message(format!("`$extends` must refer to a group, but `{}` is {kind}", show(&pending.target)))
            .primary(span, "this is not a group"),
    );
}

/// Whether `root` or any group below it still has an unexpanded `$extends`. Copying such a group now would
/// copy an incomplete one.
fn subtree_has_pending(lowered: &Lowered, root: PropId, pending: &IndexMap<PropId, Pending>) -> bool {
    pending_in_subtree(lowered, root, pending).next().is_some()
}

fn pending_in_subtree<'a>(
    lowered: &'a Lowered,
    root: PropId,
    pending: &'a IndexMap<PropId, Pending>,
) -> impl Iterator<Item = PropId> + 'a {
    let mut stack = vec![root];
    std::iter::from_fn(move || {
        while let Some(id) = stack.pop() {
            if let PropKind::Group(g) = &lowered.get(id).kind {
                stack.extend(g.members.values().copied());
                if pending.contains_key(&id) {
                    return Some(id);
                }
            }
        }
        None
    })
}

/// What is left after the loop stops making progress: groups whose target does not exist, and groups stuck in
/// cycles. A group that is only waiting on a cyclic group is not reported, the cycle already is.
fn report_leftovers(
    arena: &Arena,
    lowered: &Lowered,
    pending: &IndexMap<PropId, Pending>,
    diags: &mut impl DiagnosticSink,
) {
    let span = |p: &Pending| arena.get(lowered.get(p.extends_prop).origin).span.clone();

    let mut graph = DiGraph::<PropId, ()>::new();
    let nodes: IndexMap<PropId, _> = pending.keys().map(|g| (*g, graph.add_node(*g))).collect();

    for (group, entry) in pending {
        match resolve(lowered, &entry.target) {
            None => diags.emit(
                Diagnostic::new(DiagnosticCode::UnresolvedExtends)
                    .with_message(format!("`$extends` target `{}` does not exist", show(&entry.target)))
                    .primary(span(entry), "no group has this path"),
            ),
            Some(target) => {
                // The group waits on every unexpanded group inside its target, which includes itself when
                // the target is the group or one of its ancestors.
                for blocker in pending_in_subtree(lowered, target, pending) {
                    graph.add_edge(nodes[group], nodes[&blocker], ());
                }
            }
        }
    }

    for component in tarjan_scc(&graph) {
        if component.len() == 1 && !graph.contains_edge(component[0], component[0]) {
            continue;
        }

        let members: IndexSet<PropId> = {
            let mut m: Vec<PropId> = component.iter().map(|n| graph[*n]).collect();
            m.sort();
            m.into_iter().collect()
        };
        let mut iter = members.iter();
        let first = *iter.next().expect("components are not empty");

        let mut diag = Diagnostic::new(DiagnosticCode::ExtendsCycle)
            .with_message(format!(
                "circular `$extends`: {}",
                members.iter().map(|m| lowered.path_of(*m)).collect::<Vec<_>>().join(", ")
            ))
            .primary(span(&pending[&first]), "this group ends up extending itself");
        for member in iter {
            diag = diag.secondary(span(&pending[member]), "part of the same cycle");
        }
        diags.emit(diag);
    }
}

/// Applies the deep merge of `src` under `dst`: local content wins, and everything else is inherited.
fn merge_into(l: &mut Lowered, dst: PropId, src: PropId) {
    let (src_common, src_members) = match &l.get(src).kind {
        PropKind::Group(g) => (g.common, g.members.clone()),
        _ => return,
    };
    let (mut common, dst_members) = match &l.get(dst).kind {
        PropKind::Group(g) => (g.common, g.members.clone()),
        _ => return,
    };

    // A property the local group does not set is inherited, so a base `$type` carries over unless overridden.
    for (local, inherited) in slots(&mut common).into_iter().zip(slots_of(&src_common)) {
        if local.is_none() {
            *local = inherited.map(|p| copy(l, p, dst));
        }
    }

    // Inherited members come first in their original order, then the members only the local group has.
    let mut members = IndexMap::new();
    for (name, &inherited) in &src_members {
        let member = match dst_members.get(name) {
            Some(&local) => {
                let both_groups = matches!(l.get(local).kind, PropKind::Group(_))
                    && matches!(l.get(inherited).kind, PropKind::Group(_));
                if both_groups {
                    merge_into(l, local, inherited);
                }
                local
            }
            None => copy(l, inherited, dst),
        };
        members.insert(name.clone(), member);
    }
    for (name, local) in dst_members {
        members.entry(name).or_insert(local);
    }

    if let PropKind::Group(g) = &mut l.get_mut(dst).kind {
        g.common = common;
        g.members = members;
    }
}

fn slots(c: &mut CommonProps) -> [&mut Option<PropId>; 4] {
    [&mut c.type_, &mut c.description, &mut c.deprecated, &mut c.extensions]
}

fn slots_of(c: &CommonProps) -> [Option<PropId>; 4] {
    [c.type_, c.description, c.deprecated, c.extensions]
}

/// Copies the subtree at `src` under `parent`, giving every prop in it a fresh ID.
fn copy(l: &mut Lowered, src: PropId, parent: PropId) -> PropId {
    let origin = l.get(src).origin;
    let id = l.alloc(origin, Some(parent), Some(src));

    let kind = match l.get(src).kind.clone() {
        PropKind::Group(mut g) => {
            g.common = copy_common(l, g.common, id);
            g.extends = g.extends.map(|e| copy(l, e, id));
            g.members = g.members.into_iter().map(|(k, m)| (k, copy(l, m, id))).collect();
            PropKind::Group(g)
        }
        PropKind::Token(mut t) => {
            t.common = copy_common(l, t.common, id);
            t.body = match t.body {
                TokenBody::Value(v) => TokenBody::Value(copy(l, v, id)),
                TokenBody::Ref { prop, reference } => TokenBody::Ref {
                    prop: copy(l, prop, id),
                    reference,
                },
                TokenBody::Invalid => TokenBody::Invalid,
            };
            PropKind::Token(t)
        }
        PropKind::Value(v) => PropKind::Value(match v {
            ValueData::Array(items) => ValueData::Array(items.into_iter().map(|i| copy(l, i, id)).collect()),
            ValueData::Object(map) => {
                ValueData::Object(map.into_iter().map(|(k, v)| (k, copy(l, v, id))).collect())
            }
            other => other,
        }),
    };

    l.set_kind(id, kind);
    id
}

fn copy_common(l: &mut Lowered, mut c: CommonProps, parent: PropId) -> CommonProps {
    for slot in slots(&mut c) {
        *slot = slot.map(|p| copy(l, p, parent));
    }
    c
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        diagnostics::diagnostic::DiagnosticStore,
        ir::{
            arena::Arena,
            lower_docs,
            refs::{References, resolve_references},
        },
    };

    struct Run {
        arena: Arena,
        lowered: Lowered,
        diags: DiagnosticStore,
    }

    fn run(docs: &[&str]) -> Run {
        let (arena, mut lowered, mut diags) = lower_docs(docs);
        expand_extends(&arena, &mut lowered, &mut diags);
        Run { arena, lowered, diags }
    }

    fn codes(r: &Run) -> Vec<DiagnosticCode> {
        r.diags.diagnostics().iter().map(|d| d.code).collect()
    }

    fn names(r: &Run, path: &[&str]) -> Vec<String> {
        let id = r.lowered.member(path).unwrap_or_else(|| panic!("no member {path:?}"));
        match &r.lowered.get(id).kind {
            PropKind::Group(g) => g.members.keys().cloned().collect(),
            other => panic!("expected group, got {other:?}"),
        }
    }

    /// The `$value` number of the token at `path`.
    fn number(r: &Run, path: &[&str]) -> u64 {
        let id = r.lowered.member(path).unwrap_or_else(|| panic!("no token {path:?}"));
        let PropKind::Token(t) = &r.lowered.get(id).kind else { panic!("not a token: {path:?}") };
        let TokenBody::Value(v) = t.body else { panic!("no value") };
        match &r.arena.get(r.lowered.get(v).origin).value {
            DtcgValueSyntax::Number(n) => n.as_u64().unwrap(),
            other => panic!("expected number, got {other:?}"),
        }
    }

    fn type_of(r: &Run, path: &[&str]) -> Option<String> {
        let id = r.lowered.member(path)?;
        let common = match &r.lowered.get(id).kind {
            PropKind::Group(g) => g.common,
            PropKind::Token(t) => t.common,
            _ => return None,
        };
        match &r.arena.get(r.lowered.get(common.type_?).origin).value {
            DtcgValueSyntax::String(s) => Some(s.clone()),
            _ => None,
        }
    }

    #[test]
    fn inherits_members_and_adds_local_ones() {
        let r = run(&[r#"{"base": {"a": {"$value": 1}, "b": {"$value": 2}},
                        "derived": {"$extends": "{base}", "c": {"$value": 3}}}"#]);

        assert_eq!(names(&r, &["derived"]), ["a", "b", "c"]);
        assert_eq!(number(&r, &["derived", "a"]), 1);
        assert_eq!(number(&r, &["derived", "c"]), 3);
        // The base is untouched.
        assert_eq!(names(&r, &["base"]), ["a", "b"]);
        assert!(r.diags.diagnostics().is_empty());
    }

    #[test]
    fn local_token_replaces_the_inherited_token_whole() {
        let r = run(&[r#"{"base": {"a": {"$type": "dimension", "$value": 1}},
                        "derived": {"$extends": "{base}", "a": {"$value": 9}}}"#]);

        assert_eq!(number(&r, &["derived", "a"]), 9);
        // The inherited `$type` does not leak into the replacement.
        assert_eq!(type_of(&r, &["derived", "a"]), None);
        assert_eq!(names(&r, &["derived"]), ["a"]);
    }

    #[test]
    fn nested_groups_merge_deeply() {
        let r = run(&[r#"{"base": {"g": {"x": {"$value": 1}, "y": {"$value": 2}}, "t": {"$value": 3}},
                        "derived": {"$extends": "{base}", "g": {"y": {"$value": 20}, "z": {"$value": 30}}}}"#]);

        assert_eq!(names(&r, &["derived", "g"]), ["x", "y", "z"]);
        assert_eq!(number(&r, &["derived", "g", "x"]), 1);
        assert_eq!(number(&r, &["derived", "g", "y"]), 20);
        assert_eq!(number(&r, &["derived", "g", "z"]), 30);
        assert_eq!(number(&r, &["derived", "t"]), 3);
    }

    #[test]
    fn inherited_members_come_first_in_base_order() {
        let r = run(&[r#"{"base": {"a": {"$value": 1}, "b": {"$value": 2}},
                        "derived": {"$extends": "{base}", "z": {"$value": 0}, "b": {"$value": 5}}}"#]);
        assert_eq!(names(&r, &["derived"]), ["a", "b", "z"]);
    }

    #[test]
    fn group_properties_are_inherited_unless_set_locally() {
        let r = run(&[r#"{"base": {"$type": "color", "$description": "base", "a": {"$value": 1}},
                        "d1": {"$extends": "{base}"}, "d2": {"$extends": "{base}", "$type": "dimension"}}"#]);

        assert_eq!(type_of(&r, &["d1"]), Some("color".into()));
        assert_eq!(type_of(&r, &["d2"]), Some("dimension".into()));
        let PropKind::Group(g) = &r.lowered.get(r.lowered.member(&["d2"]).unwrap()).kind else { panic!() };
        assert!(g.common.description.is_some());
    }

    #[test]
    fn copies_get_fresh_ids_and_remember_where_they_came_from() {
        let r = run(&[r#"{"base": {"a": {"$value": {"k": [1]}}}, "derived": {"$extends": "{base}"}}"#]);

        let original = r.lowered.member(&["base", "a"]).unwrap();
        let copy = r.lowered.member(&["derived", "a"]).unwrap();
        assert_ne!(original, copy);
        assert_eq!(r.lowered.get(copy).inherited_from, Some(original));
        assert_eq!(r.lowered.get(copy).origin, r.lowered.get(original).origin);
        assert_eq!(r.lowered.get(copy).parent, r.lowered.member(&["derived"]));
        assert_eq!(r.lowered.get(original).inherited_from, None);

        // Every prop inside the copy is new too, and the parent links are consistent.
        for (_, child) in r.lowered.keyed_children(copy) {
            assert_eq!(r.lowered.get(child).parent, Some(copy));
            assert!(r.lowered.get(child).inherited_from.is_some());
        }
    }

    #[test]
    fn chains_expand_in_dependency_order_whatever_the_declaration_order() {
        // `c` is declared first but needs `b`, which needs `a`.
        let r = run(&[r#"{"c": {"$extends": "{b}", "z": {"$value": 3}},
                        "b": {"$extends": "{a}", "y": {"$value": 2}},
                        "a": {"x": {"$value": 1}}}"#]);

        assert_eq!(names(&r, &["b"]), ["x", "y"]);
        assert_eq!(names(&r, &["c"]), ["x", "y", "z"]);
        assert!(r.diags.diagnostics().is_empty());
    }

    #[test]
    fn nested_extends_inside_the_target_are_expanded_before_copying() {
        let r = run(&[r#"{"user": {"$extends": "{lib}"},
                        "lib": {"inner": {"$extends": "{base}", "own": {"$value": 2}}},
                        "base": {"x": {"$value": 1}}}"#]);

        assert_eq!(names(&r, &["user", "inner"]), ["x", "own"]);
    }

    #[test]
    fn a_target_path_can_go_through_an_extended_group() {
        let r = run(&[r#"{"user": {"$extends": "{derived.sub}"},
                        "derived": {"$extends": "{base}"},
                        "base": {"sub": {"x": {"$value": 1}}}}"#]);

        assert_eq!(names(&r, &["user"]), ["x"]);
        assert!(r.diags.diagnostics().is_empty());
    }

    #[test]
    fn extends_accepts_a_ref_object() {
        let r = run(&[r##"{"base": {"a": {"$value": 1}}, "derived": {"$extends": {"$ref": "#/base"}}}"##]);
        assert_eq!(names(&r, &["derived"]), ["a"]);
        assert!(r.diags.diagnostics().is_empty());
    }

    #[test]
    fn extends_follows_the_merged_tree() {
        // The second document overrides `a` in the base, and the extending group sees the override.
        let r = run(&[
            r#"{"base": {"a": {"$value": 1}}, "derived": {"$extends": "{base}"}}"#,
            r#"{"base": {"a": {"$value": 2}, "b": {"$value": 3}}}"#,
        ]);

        assert_eq!(number(&r, &["derived", "a"]), 2);
        assert_eq!(names(&r, &["derived"]), ["a", "b"]);
    }

    #[test]
    fn extends_cycles_are_reported_once_with_every_member() {
        let r = run(&[r#"{"a": {"$extends": "{b}"}, "b": {"$extends": "{c}"}, "c": {"$extends": "{a}"}}"#]);

        assert_eq!(codes(&r), [DiagnosticCode::ExtendsCycle]);
        assert_eq!(r.diags.diagnostics()[0].labels.len(), 3);
    }

    #[test]
    fn extending_yourself_or_your_ancestor_is_a_cycle() {
        for doc in [
            r#"{"a": {"$extends": "{a}"}}"#,
            r#"{"parent": {"child": {"$extends": "{parent}"}}}"#,
            r#"{"parent": {"$extends": "{parent.child}", "child": {"x": {"$value": 1}}}}"#,
        ] {
            let r = run(&[doc]);
            // The last case is not a cycle: the child is complete, so the parent can inherit it.
            let expected: &[DiagnosticCode] = if doc.contains("parent.child") { &[] } else { &[DiagnosticCode::ExtendsCycle] };
            assert_eq!(codes(&r), expected, "doc: {doc}");
        }
    }

    #[test]
    fn groups_waiting_on_a_cycle_are_not_reported_again() {
        let r = run(&[r#"{"a": {"$extends": "{b}"}, "b": {"$extends": "{a}"}, "c": {"$extends": "{a}"}}"#]);
        assert_eq!(codes(&r), [DiagnosticCode::ExtendsCycle]);
    }

    #[test]
    fn unresolved_targets_are_reported() {
        let r = run(&[r##"{"a": {"$extends": "{missing}"}, "b": {"$extends": {"$ref": "#/nope"}}}"##]);
        assert_eq!(codes(&r), [DiagnosticCode::UnresolvedExtends; 2]);
    }

    #[test]
    fn extending_a_token_or_a_property_is_rejected() {
        let r = run(&[r##"{"t": {"$value": 1}, "a": {"$extends": "{t}"}, "b": {"$extends": {"$ref": "#/t/$value"}}}"##]);
        assert_eq!(codes(&r), [DiagnosticCode::InvalidExtendsTarget; 2]);
        // Nothing was inherited.
        assert!(names(&r, &["a"]).is_empty());
    }

    #[test]
    fn malformed_and_unsupported_extends_are_reported() {
        let r = run(&[r##"{"a": {"$extends": "base"}, "b": {"$extends": "{x..y}"}, "c": {"$extends": {"$ref": "other.json#/x"}},
                         "d": {"$extends": {"$ref": 5}}, "e": {"$extends": {"$ref": "C:\\a.json"}}}"##]);

        assert_eq!(codes(&r), [DiagnosticCode::InvalidReference; 5]);
        assert!(r.diags.diagnostics()[4].help.is_some());
    }

    #[test]
    fn nothing_to_expand_changes_nothing() {
        let (arena, mut lowered, mut diags) = lower_docs(&[r#"{"a": {"x": {"$value": 1}}}"#]);
        let before = lowered.len();
        expand_extends(&arena, &mut lowered, &mut diags);
        assert_eq!(lowered.len(), before);
    }

    // The reason for the whole design: inherited tokens are real nodes the reference graph can tell apart.
    #[test]
    fn inherited_tokens_are_resolved_as_separate_holders() {
        let (arena, mut lowered, mut diags) = lower_docs(&[r#"{"base": {"x": {"$value": "{base.y}"}, "y": {"$value": 1}},
                        "derived": {"$extends": "{base}", "y": {"$value": 2}}}"#]);
        expand_extends(&arena, &mut lowered, &mut diags);
        let refs: References = resolve_references(&arena, &lowered, &mut diags);

        assert!(diags.diagnostics().is_empty());
        let base_x = lowered.resolve_pointer(&JsonPointer::from_segments(["base", "x", "$value"])).unwrap();
        let derived_x = lowered.resolve_pointer(&JsonPointer::from_segments(["derived", "x", "$value"])).unwrap();
        assert_ne!(base_x, derived_x);
        // Aliases are absolute, so the inherited copy still points at `base.y`, as with a JSON Schema `$ref`.
        assert_eq!(refs.target(base_x), refs.target(derived_x));
        assert_eq!(refs.targets.len(), 2);
    }

    #[test]
    fn extends_can_create_a_reference_cycle_that_the_graph_finds() {
        // `derived.x` is inherited from `base.x`, which refers to the group that now contains it.
        let (arena, mut lowered, mut diags) = lower_docs(&[r##"{"base": {"x": {"$ref": "#/derived"}}, "derived": {"$extends": "{base}"}}"##]);
        expand_extends(&arena, &mut lowered, &mut diags);
        assert!(diags.diagnostics().is_empty());
        resolve_references(&arena, &lowered, &mut diags);
        assert_eq!(diags.diagnostics().len(), 1);
        assert_eq!(diags.diagnostics()[0].code, DiagnosticCode::ReferenceCycle);
    }
}
