//! The `refs` module builds the reference graph over the lowered tree and checks it.
//!
//! A *holder* is a prop that points at another prop:
//! - a token whose body is `$ref`,
//! - a string inside a token's `$value` that is exactly an alias, such as `"{color.blue}"`,
//! - an object inside a token's `$value` that is a `$ref` object, such as `{"$ref": "#/color/blue/$value"}`.
//!
//! Two kinds of edge go into one graph over every [`PropId`]:
//! - `Refers`: holder to target. The holder needs the target's value first.
//! - `Contains`: a prop to each of its children. A container is only complete once its children are.
//!
//! Containment edges are what make a reference to your own ancestor a cycle. `a: {"$ref": "#"}` would need the
//! whole document, which needs `a`, which needs the document. A single SCC pass then finds every cycle, whether
//! it is made of aliases, `$ref`s, containment, or a mix.
//!
//! Aliases only target tokens (the spec reserves `{...}` for complete token values), while `$ref` may target any
//! prop: a group, a token, a `$`-property, or something deep inside a value. File-qualified references are not
//! resolved yet and are reported.

use indexmap::IndexMap;
use petgraph::{
    algo::tarjan_scc,
    graph::{DiGraph, NodeIndex},
    visit::EdgeRef,
};

use crate::{
    ast::reference::JsonReference,
    diagnostics::{
        code::DiagnosticCode,
        diagnostic::{Diagnostic, DiagnosticSink},
    },
    ir::{
        arena::Arena,
        lower::{Lowered, PropId, PropKind, TokenBody, ValueData},
        node::DtcgValueSyntax,
    },
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Edge {
    Contains,
    Refers,
}

#[derive(Debug)]
pub struct References {
    /// One node per prop, and `NodeIndex::new(i)` is `PropId(i)`.
    pub graph: DiGraph<PropId, Edge>,
    /// Every holder that resolved, mapped to its target.
    pub targets: IndexMap<PropId, PropId>,
}

impl References {
    pub fn target(&self, holder: PropId) -> Option<PropId> {
        self.targets.get(&holder).copied()
    }
}

/// Parses an alias like `{color.brand.$root}` into its path segments.
/// Returns `None` if the string is not shaped like an alias. Use [`is_alias_like`] to tell "not an alias" from
/// "a broken alias".
pub fn parse_alias(s: &str) -> Option<Vec<String>> {
    let inner = s.strip_prefix('{')?.strip_suffix('}')?;
    // Names cannot contain `{`, `}` or `.`, so any of them left over means the alias is malformed.
    if inner.contains(['{', '}']) {
        return None;
    }
    let segments: Vec<String> = inner.split('.').map(str::to_string).collect();
    segments.iter().all(|s| !s.is_empty()).then_some(segments)
}

/// Whether a string starts with `{` and ends with `}`, so the user probably meant it to be an alias.
pub fn is_alias_like(s: &str) -> bool {
    s.starts_with('{') && s.ends_with('}')
}

/// What a holder points at, before it is looked up.
#[derive(Debug)]
enum Reference {
    Alias(Vec<String>),
    Pointer(JsonReference),
}

struct Holder {
    prop: PropId,
    /// The prop whose span the diagnostic should underline: the `$ref` string for a `$ref` token, otherwise the holder.
    at: PropId,
    reference: Reference,
}

pub fn resolve_references(
    arena: &Arena,
    lowered: &Lowered,
    diags: &mut impl DiagnosticSink,
) -> References {
    let mut graph = DiGraph::<PropId, Edge>::with_capacity(lowered.len(), lowered.len());
    for i in 0..lowered.len() {
        graph.add_node(PropId(i as u32));
    }
    let node = |id: PropId| NodeIndex::new(id.0 as usize);

    for i in 0..lowered.len() as u32 {
        let id = PropId(i);
        for child in lowered.children(id) {
            graph.add_edge(node(id), node(child), Edge::Contains);
        }
    }

    let mut targets = IndexMap::new();
    for holder in collect_holders(arena, lowered, diags) {
        let span = arena.get(lowered.get(holder.at).origin).span.clone();

        match find_target(lowered, &holder.reference) {
            Ok(target) => {
                graph.add_edge(node(holder.prop), node(target), Edge::Refers);
                targets.insert(holder.prop, target);
            }
            Err(failure) => {
                let mut diag = Diagnostic::new(failure.code)
                    .with_message(failure.message)
                    .primary(span, failure.label);
                if let Some(help) = failure.help {
                    diag = diag.help(help);
                }
                diags.emit(diag);
            }
        }
    }

    report_cycles(arena, lowered, &graph, diags);

    References { graph, targets }
}

struct Failure {
    code: DiagnosticCode,
    message: String,
    label: String,
    help: Option<String>,
}

fn find_target(lowered: &Lowered, reference: &Reference) -> Result<PropId, Failure> {
    match reference {
        Reference::Alias(segments) => {
            let shown = format!("{{{}}}", segments.join("."));
            let Some(target) = lowered.member_path(segments) else {
                return Err(Failure {
                    code: DiagnosticCode::UnresolvedReference,
                    message: format!("`{shown}` does not refer to anything"),
                    label: "no token or group has this path".into(),
                    help: None,
                });
            };
            if matches!(lowered.get(target).kind, PropKind::Token(_)) {
                return Ok(target);
            }

            // An alias to a group is a common mistake when the group has a `$root` token.
            let help = lowered
                .member_path(&[segments.as_slice(), &["$root".to_string()]].concat())
                .map(|_| format!("did you mean `{{{}.$root}}`?", segments.join(".")));
            Err(Failure {
                code: DiagnosticCode::InvalidReferenceTarget,
                message: format!("`{shown}` refers to a group, but an alias must refer to a token"),
                label: "this is a group".into(),
                help,
            })
        }
        Reference::Pointer(reference) => {
            if let Some(document) = reference.document() {
                return Err(Failure {
                    code: DiagnosticCode::InvalidReference,
                    message: format!(
                        "references to other files (`{}`) are not supported yet",
                        document.path().unwrap_or_default()
                    ),
                    label: "only `#/...` references are resolved".into(),
                    help: None,
                });
            }
            lowered.resolve_pointer(reference.fragment()).ok_or_else(|| Failure {
                code: DiagnosticCode::UnresolvedReference,
                message: format!("`#{}` does not point to anything", reference.fragment().to_pointer_string()),
                label: "no such path in the merged tokens".into(),
                help: None,
            })
        }
    }
}

fn collect_holders(arena: &Arena, lowered: &Lowered, diags: &mut impl DiagnosticSink) -> Vec<Holder> {
    let mut out = Vec::new();

    for i in 0..lowered.len() as u32 {
        let id = PropId(i);
        let PropKind::Token(token) = &lowered.get(id).kind else {
            continue;
        };
        match &token.body {
            TokenBody::Value(value) => scan_value(arena, lowered, *value, &mut out, diags),
            TokenBody::Ref { prop, reference } => out.push(Holder {
                prop: id,
                at: *prop,
                reference: Reference::Pointer(reference.clone()),
            }),
            TokenBody::Invalid => {}
        }
    }

    out
}

/// Looks for holders inside the plain JSON of a token's `$value`.
fn scan_value(
    arena: &Arena,
    lowered: &Lowered,
    id: PropId,
    out: &mut Vec<Holder>,
    diags: &mut impl DiagnosticSink,
) {
    let PropKind::Value(value) = &lowered.get(id).kind else {
        return;
    };
    let raw = arena.get(lowered.get(id).origin);

    match value {
        ValueData::Scalar => {
            let DtcgValueSyntax::String(text) = &raw.value else {
                return;
            };
            if !is_alias_like(text) {
                return;
            }
            match parse_alias(text) {
                Some(segments) => out.push(Holder {
                    prop: id,
                    at: id,
                    reference: Reference::Alias(segments),
                }),
                None => diags.emit(
                    Diagnostic::new(DiagnosticCode::InvalidReference)
                        .with_message(format!("`{text}` is not a valid alias"))
                        .primary(raw.span.clone(), "expected `{group.token}`")
                        .help("names are separated by `.` and cannot be empty or contain `{` or `}`"),
                ),
            }
        }
        ValueData::Array(items) => {
            for item in items {
                scan_value(arena, lowered, *item, out, diags);
            }
        }
        ValueData::Object(map) => {
            if let Some(&reference_prop) = map.get("$ref") {
                scan_ref_object(arena, lowered, id, reference_prop, out, diags);
                return;
            }
            for child in map.values() {
                scan_value(arena, lowered, *child, out, diags);
            }
        }
        ValueData::Invalid => {}
    }
}

fn scan_ref_object(
    arena: &Arena,
    lowered: &Lowered,
    holder: PropId,
    reference_prop: PropId,
    out: &mut Vec<Holder>,
    diags: &mut impl DiagnosticSink,
) {
    let raw = arena.get(lowered.get(reference_prop).origin);
    let DtcgValueSyntax::String(text) = &raw.value else {
        if !matches!(raw.value, DtcgValueSyntax::Invalid) {
            diags.emit(
                Diagnostic::new(DiagnosticCode::InvalidPropertyShape)
                    .with_message("`$ref` must be a string")
                    .primary(raw.span.clone(), "expected a JSON pointer string"),
            );
        }
        return;
    };

    match JsonReference::from_str(text) {
        Ok(reference) => out.push(Holder {
            prop: holder,
            at: reference_prop,
            reference: Reference::Pointer(reference),
        }),
        Err(err) => {
            let mut diag = Diagnostic::new(DiagnosticCode::InvalidReference)
                .with_message(format!("`{text}` is not a valid reference"))
                .primary(raw.span.clone(), format!("{err:?}"));
            if let Some(hint) = err.hint() {
                diag = diag.help(hint);
            }
            diags.emit(diag);
        }
    }
}

fn report_cycles(
    arena: &Arena,
    lowered: &Lowered,
    graph: &DiGraph<PropId, Edge>,
    diags: &mut impl DiagnosticSink,
) {
    for component in tarjan_scc(graph) {
        // A single node is only a cycle if it points at itself.
        let cyclic = component.len() > 1 || graph.contains_edge(component[0], component[0]);
        if !cyclic {
            continue;
        }

        let mut members: Vec<PropId> = component.iter().map(|n| graph[*n]).collect();
        members.sort();

        // Containment alone is a tree, so every cycle has at least one holder. Those are the useful labels.
        let holders: Vec<PropId> = members
            .iter()
            .copied()
            .filter(|m| {
                graph
                    .edges(NodeIndex::new(m.0 as usize))
                    .any(|e| *e.weight() == Edge::Refers && component.contains(&e.target()))
            })
            .collect();
        let Some((&first, rest)) = holders.split_first() else {
            continue;
        };

        let span = |id: PropId| arena.get(lowered.get(id).origin).span.clone();
        let mut diag = Diagnostic::new(DiagnosticCode::ReferenceCycle)
            .with_message(format!(
                "circular reference: {}",
                holders.iter().map(|h| lowered.path_of(*h)).collect::<Vec<_>>().join(", ")
            ))
            .primary(span(first), "this reference leads back to itself");
        for holder in rest {
            diag = diag.secondary(span(*holder), "part of the same cycle");
        }
        diags.emit(diag);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{diagnostics::diagnostic::DiagnosticStore, ir::lower_docs};

    struct Run {
        lowered: Lowered,
        refs: References,
        diags: DiagnosticStore,
    }

    fn run(docs: &[&str]) -> Run {
        let (arena, lowered, mut diags) = lower_docs(docs);
        let refs = resolve_references(&arena, &lowered, &mut diags);
        Run { lowered, refs, diags }
    }

    fn codes(r: &Run) -> Vec<DiagnosticCode> {
        r.diags.diagnostics().iter().map(|d| d.code).collect()
    }

    fn id(r: &Run, path: &[&str]) -> PropId {
        r.lowered.member(path).unwrap_or_else(|| panic!("no member {path:?}"))
    }

    /// The path of what the holder at `p` points to.
    fn target_path(r: &Run, holder: PropId) -> String {
        r.lowered.path_of(r.refs.target(holder).expect("holder resolved"))
    }

    #[test]
    fn alias_value_resolves_to_the_token() {
        let r = run(&[r#"{"a": {"$value": "{b}"}, "b": {"$value": 1}}"#]);

        let holder = r.lowered.resolve_pointer(&"/a/$value".parse_pointer()).unwrap();
        assert_eq!(r.refs.target(holder), Some(id(&r, &["b"])));
        assert!(r.diags.diagnostics().is_empty());
    }

    #[test]
    fn alias_uses_dotted_paths_and_root() {
        let r = run(&[r#"{"c": {"brand": {"$root": {"$value": 1}, "dark": {"$value": 2}},
                         "x": {"$value": "{c.brand.$root}"}, "y": {"$value": "{c.brand.dark}"}}}"#]);

        assert_eq!(target_path(&r, r.lowered.resolve_pointer(&"/c/x/$value".parse_pointer()).unwrap()), "#/c/brand/$root");
        assert_eq!(target_path(&r, r.lowered.resolve_pointer(&"/c/y/$value".parse_pointer()).unwrap()), "#/c/brand/dark");
        assert!(r.diags.diagnostics().is_empty());
    }

    #[test]
    fn ref_token_resolves_to_any_prop() {
        let r = run(&[r##"{"a": {"$ref": "#/b"}, "c": {"$ref": "#/b/$value/k/1"}, "g": {"t": {"$ref": "#/h"}}, "h": {"i": {"$value": 1}},
                         "b": {"$value": {"k": [1, 2]}}}"##]);

        assert_eq!(r.refs.target(id(&r, &["a"])), Some(id(&r, &["b"])));
        assert_eq!(target_path(&r, id(&r, &["c"])), "#/b/$value/k/1");
        // A `$ref` may target a group, unlike an alias.
        assert_eq!(r.refs.target(id(&r, &["g", "t"])), Some(id(&r, &["h"])));
        assert!(r.diags.diagnostics().is_empty());
    }

    #[test]
    fn ref_to_a_dollar_property() {
        let r = run(&[r##"{"g": {"$type": "color", "t": {"$ref": "#/g/$type"}}}"##]);
        assert_eq!(target_path(&r, id(&r, &["g", "t"])), "#/g/$type");
    }

    #[test]
    fn holders_inside_composite_values() {
        let r = run(&[r##"{"x": {"$value": 1}, "s": {"$value": {"color": "{x}", "list": ["{x}", 2, {"$ref": "#/x/$value"}]}}}"##]);

        assert_eq!(r.refs.targets.len(), 3);
        assert!(r.diags.diagnostics().is_empty());
    }

    #[test]
    fn strings_outside_a_token_value_are_never_aliases() {
        let r = run(&[r#"{"g": {"$description": "{nope}", "$extensions": {"x": "{nope}"},
                         "t": {"$description": "{nope}", "$value": "plain {text} here"}}}"#]);

        assert!(r.refs.targets.is_empty());
        assert!(r.diags.diagnostics().is_empty());
    }

    #[test]
    fn unresolved_alias_and_ref_are_reported() {
        let r = run(&[r##"{"a": {"$value": "{missing.token}"}, "b": {"$ref": "#/missing"}, "c": {"$ref": "#/a/$value/x"}}"##]);

        assert_eq!(codes(&r), [DiagnosticCode::UnresolvedReference; 3]);
        assert!(r.refs.targets.is_empty());
    }

    #[test]
    fn alias_to_a_group_is_rejected_with_a_root_hint() {
        let r = run(&[r#"{"g": {"$root": {"$value": 1}}, "h": {"x": {"$value": 1}}, "a": {"$value": "{g}"}, "b": {"$value": "{h}"}}"#]);

        assert_eq!(codes(&r), [DiagnosticCode::InvalidReferenceTarget; 2]);
        assert!(r.diags.diagnostics()[0].help.as_deref().unwrap().contains("{g.$root}"));
        assert!(r.diags.diagnostics()[1].help.is_none());
    }

    #[test]
    fn malformed_aliases_are_reported() {
        for bad in ["{a..b}", "{}", "{a{b}", "{.a}"] {
            let r = run(&[&format!(r#"{{"a": {{"$value": "{bad}"}}}}"#)]);
            assert_eq!(codes(&r), [DiagnosticCode::InvalidReference], "alias: {bad}");
        }
    }

    #[test]
    fn bad_and_unsupported_refs_in_values_are_reported() {
        let r = run(&[r#"{"a": {"$value": {"$ref": 5}}, "b": {"$value": {"$ref": "C:\\x\\y.json#/a"}},
                         "c": {"$value": {"$ref": "other.json#/a"}}, "d": {"$ref": "other.json#/a"}}"#]);

        assert_eq!(
            codes(&r),
            [
                DiagnosticCode::InvalidPropertyShape,
                DiagnosticCode::InvalidReference,
                DiagnosticCode::InvalidReference,
                DiagnosticCode::InvalidReference
            ]
        );
        assert!(r.diags.diagnostics()[1].help.is_some());
    }

    #[test]
    fn chains_are_not_cycles() {
        let r = run(&[r##"{"a": {"$value": "{b}"}, "b": {"$ref": "#/c"}, "c": {"$value": 1}}"##]);
        assert!(r.diags.diagnostics().is_empty());
        assert_eq!(r.refs.targets.len(), 2);
    }

    #[test]
    fn referencing_a_sibling_in_the_same_group_is_not_a_cycle() {
        let r = run(&[r##"{"g": {"a": {"$value": 1}, "b": {"$ref": "#/g/a"}}}"##]);
        assert!(r.diags.diagnostics().is_empty());
    }

    #[test]
    fn alias_cycle_is_reported_once_with_every_member() {
        let r = run(&[r#"{"a": {"$value": "{b}"}, "b": {"$value": "{c}"}, "c": {"$value": "{a}"}}"#]);

        assert_eq!(codes(&r), [DiagnosticCode::ReferenceCycle]);
        let d = &r.diags.diagnostics()[0];
        assert_eq!(d.labels.len(), 3);
        assert!(d.message.contains("#/a/$value") && d.message.contains("#/c/$value"));
    }

    #[test]
    fn mixed_alias_and_ref_cycle_is_found() {
        let r = run(&[r##"{"a": {"$value": "{b}"}, "b": {"$ref": "#/a"}}"##]);
        assert_eq!(codes(&r), [DiagnosticCode::ReferenceCycle]);
    }

    #[test]
    fn self_references_are_cycles() {
        for doc in [
            r##"{"a": {"$ref": "#/a"}}"##,
            r#"{"a": {"$value": "{a}"}}"#,
            r##"{"a": {"$value": {"x": 1, "y": {"$ref": "#/a/$value"}}}}"##,
        ] {
            let r = run(&[doc]);
            assert_eq!(codes(&r), [DiagnosticCode::ReferenceCycle], "doc: {doc}");
        }
    }

    #[test]
    fn a_ref_to_an_ancestor_is_a_cycle_through_containment() {
        let r = run(&[r##"{"g": {"t": {"$ref": "#/g"}}}"##]);
        assert_eq!(codes(&r), [DiagnosticCode::ReferenceCycle]);

        let r = run(&[r##"{"t": {"$ref": "#"}}"##]);
        assert_eq!(codes(&r), [DiagnosticCode::ReferenceCycle]);
    }

    #[test]
    fn two_independent_cycles_are_reported_separately() {
        let r = run(&[r#"{"a": {"$value": "{b}"}, "b": {"$value": "{a}"}, "c": {"$value": "{d}"}, "d": {"$value": "{c}"}}"#]);
        assert_eq!(codes(&r), [DiagnosticCode::ReferenceCycle; 2]);
    }

    #[test]
    fn later_layer_changes_what_a_reference_points_at() {
        let r = run(&[
            r##"{"a": {"$ref": "#/b"}, "b": {"$value": 1}}"##,
            r##"{"b": {"$ref": "#/a"}}"##,
        ]);
        // The second document turns `b` into a reference back to `a`.
        assert_eq!(codes(&r), [DiagnosticCode::ReferenceCycle]);
    }

    #[test]
    fn graph_has_one_node_per_prop_and_both_edge_kinds() {
        let r = run(&[r#"{"a": {"$value": "{b}"}, "b": {"$value": 1}}"#]);

        assert_eq!(r.refs.graph.node_count(), r.lowered.len());
        let refers = r.refs.graph.edge_weights().filter(|e| **e == Edge::Refers).count();
        let contains = r.refs.graph.edge_weights().filter(|e| **e == Edge::Contains).count();
        assert_eq!(refers, 1);
        // root->a, root->b, a->$value, b->$value
        assert_eq!(contains, 4);
    }

    trait ParsePointer {
        fn parse_pointer(&self) -> crate::ast::reference::JsonPointer;
    }
    impl ParsePointer for str {
        fn parse_pointer(&self) -> crate::ast::reference::JsonPointer {
            crate::ast::reference::JsonPointer::parse_pointer(self).unwrap()
        }
    }
}
