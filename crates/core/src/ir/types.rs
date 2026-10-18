//! The `types` module decides the `$type` of every token.
//!
//! Spec order (DTCG 2025.10, 5.2.2 and 6.7.3):
//! 1. the token's own `$type`,
//! 2. if its value is a reference, the resolved type of the referenced token,
//! 3. the `$type` of the closest parent group (groups already contain what `$extends` copied into them),
//! 4. otherwise the token is invalid. The type is never guessed from the value.
//!
//! This runs after the cycle gate, so following references always terminates. The result is a side table keyed by
//! token: lowering stays immutable, and each answer remembers which prop it came from for diagnostics.
//! The type is kept as a string. Whether the name is a known DTCG type is for per-type value validation.

use indexmap::IndexMap;

use crate::{
    diagnostics::{
        code::DiagnosticCode,
        diagnostic::{Diagnostic, DiagnosticSink},
    },
    ir::{
        arena::Arena,
        lower::{Lowered, PropId, PropKind, TokenBody},
        node::DtcgValueSyntax,
        refs::{References, is_alias_like},
    },
};

/// Where a token's type came from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TypeSource {
    /// The token's own `$type` prop.
    Explicit(PropId),
    /// The referenced token whose type was used.
    FromReference(PropId),
    /// The `$type` prop of a parent group.
    FromGroup(PropId),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolved {
    Known {
        name: String,
        source: TypeSource,
    },
    /// No usable type. The reason was reported, either on this token or on the one it depends on.
    Unresolvable,
}

#[derive(Debug, Default)]
pub struct TypeMap {
    types: IndexMap<PropId, Resolved>,
}

impl TypeMap {
    pub fn get(&self, token: PropId) -> Option<&Resolved> {
        self.types.get(&token)
    }

    /// The type name of a token, if it has one.
    pub fn name(&self, token: PropId) -> Option<&str> {
        match self.types.get(&token)? {
            Resolved::Known { name, .. } => Some(name),
            Resolved::Unresolvable => None,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (PropId, &Resolved)> {
        self.types.iter().map(|(k, v)| (*k, v))
    }
}

enum State {
    InProgress,
    Done(Resolved),
}

struct Resolver<'a> {
    arena: &'a Arena,
    lowered: &'a Lowered,
    references: &'a References,
    states: IndexMap<PropId, State>,
    /// Tokens that had no type and were not waiting on a broken reference.
    untyped: Vec<PropId>,
}

pub fn resolve_types(
    arena: &Arena,
    lowered: &Lowered,
    references: &References,
    diags: &mut impl DiagnosticSink,
) -> TypeMap {
    let mut resolver = Resolver {
        arena,
        lowered,
        references,
        states: IndexMap::new(),
        untyped: Vec::new(),
    };

    let mut types = IndexMap::new();
    for i in 0..lowered.len() as u32 {
        let id = PropId(i);
        // Tokens with an invalid body were already reported.
        if matches!(&lowered.get(id).kind, PropKind::Token(t) if !matches!(t.body, TokenBody::Invalid))
        {
            types.insert(id, resolver.resolve(id));
        }
    }

    for token in resolver.untyped {
        diags.emit(
            Diagnostic::new(DiagnosticCode::UnresolvedType)
                .with_message(format!("cannot determine the type of `{}`", lowered.path_of(token)))
                .primary(arena.get(lowered.get(token).origin).span.clone(), "no `$type` here, in a parent group, or on a referenced token")
                .help("set `$type` on the token or on a parent group. A type is never guessed from the value"),
        );
    }

    TypeMap { types }
}

impl Resolver<'_> {
    fn resolve(&mut self, token: PropId) -> Resolved {
        match self.states.get(&token) {
            Some(State::Done(r)) => return r.clone(),
            // Cannot happen after the cycle gate, but this keeps the function total.
            Some(State::InProgress) => return Resolved::Unresolvable,
            None => {}
        }
        self.states.insert(token, State::InProgress);

        let result = self.compute(token);
        self.states.insert(token, State::Done(result.clone()));
        result
    }

    fn compute(&mut self, token: PropId) -> Resolved {
        let PropKind::Token(data) = &self.lowered.get(token).kind else {
            return Resolved::Unresolvable;
        };

        if let Some(prop) = data.common.type_ {
            return self.known(prop, TypeSource::Explicit(prop));
        }

        let (holder, is_reference) = match &data.body {
            TokenBody::Ref { .. } => (token, true),
            TokenBody::Value(v) => (*v, self.is_alias(*v)),
            TokenBody::Invalid => return Resolved::Unresolvable,
        };

        if is_reference {
            match self.references.target(holder) {
                Some(target) if matches!(self.lowered.get(target).kind, PropKind::Token(_)) => {
                    return match self.resolve(target) {
                        Resolved::Known { name, .. } => Resolved::Known {
                            name,
                            source: TypeSource::FromReference(target),
                        },
                        Resolved::Unresolvable => Resolved::Unresolvable,
                    };
                }
                // The reference is broken, and that was already reported.
                None => return Resolved::Unresolvable,
                // A `$ref` into a group or a sub-property has no token type to take, so the groups decide.
                Some(_) => {}
            }
        }

        let mut parent = self.lowered.get(token).parent;
        while let Some(id) = parent {
            if let PropKind::Group(g) = &self.lowered.get(id).kind {
                if let Some(prop) = g.common.type_ {
                    return self.known(prop, TypeSource::FromGroup(prop));
                }
            }
            parent = self.lowered.get(id).parent;
        }

        self.untyped.push(token);
        Resolved::Unresolvable
    }

    fn known(&self, type_prop: PropId, source: TypeSource) -> Resolved {
        match &self.arena.get(self.lowered.get(type_prop).origin).value {
            DtcgValueSyntax::String(name) => Resolved::Known {
                name: name.clone(),
                source,
            },
            // A non-string `$type` was reported by lowering.
            _ => Resolved::Unresolvable,
        }
    }

    fn is_alias(&self, value: PropId) -> bool {
        matches!(&self.arena.get(self.lowered.get(value).origin).value,
            DtcgValueSyntax::String(s) if is_alias_like(s))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        diagnostics::diagnostic::DiagnosticStore,
        ir::{
            merge::LayerOrigin,
            pipeline::{Analyzed, Layer, analyze},
        },
        source::SourceIdGenerator,
    };

    fn run(docs: &[&str]) -> (Analyzed, DiagnosticStore) {
        let mut ids = SourceIdGenerator::default();
        let trees: Vec<_> = docs
            .iter()
            .map(|d| {
                let (tree, errors) = json_spanned::parse_json(d);
                assert!(errors.is_empty());
                (ids.generate(), tree.unwrap())
            })
            .collect();
        let layers: Vec<_> = trees
            .iter()
            .map(|(s, t)| Layer {
                source: *s,
                origin: LayerOrigin::File(*s),
                tree: t,
            })
            .collect();
        let mut diags = DiagnosticStore::new();
        let analyzed = analyze(&layers, &mut diags).expect("no cycles");
        (analyzed, diags)
    }

    fn ty(a: &Analyzed, path: &[&str]) -> Option<String> {
        let id = a
            .lowered
            .member(path)
            .unwrap_or_else(|| panic!("no token {path:?}"));
        a.types.name(id).map(str::to_string)
    }

    fn source(a: &Analyzed, path: &[&str]) -> TypeSource {
        match a.types.get(a.lowered.member(path).unwrap()) {
            Some(Resolved::Known { source, .. }) => *source,
            other => panic!("no type: {other:?}"),
        }
    }

    fn codes(d: &DiagnosticStore) -> Vec<DiagnosticCode> {
        d.diagnostics().iter().map(|d| d.code).collect()
    }

    #[test]
    fn explicit_type_wins_over_the_group() {
        let (a, d) =
            run(&[r#"{"g": {"$type": "color", "t": {"$type": "dimension", "$value": 1}}}"#]);
        assert_eq!(ty(&a, &["g", "t"]).as_deref(), Some("dimension"));
        assert!(matches!(source(&a, &["g", "t"]), TypeSource::Explicit(_)));
        assert!(d.diagnostics().is_empty());
    }

    #[test]
    fn the_closest_group_wins() {
        let (a, _) = run(&[
            r#"{"a": {"$type": "color", "b": {"$type": "dimension", "t": {"$value": 1}}, "u": {"$value": 2}}}"#,
        ]);
        assert_eq!(ty(&a, &["a", "b", "t"]).as_deref(), Some("dimension"));
        assert_eq!(ty(&a, &["a", "u"]).as_deref(), Some("color"));
        assert!(matches!(source(&a, &["a", "u"]), TypeSource::FromGroup(_)));
    }

    #[test]
    fn an_alias_takes_the_type_of_its_target_before_its_group() {
        let (a, d) = run(&[r#"{"base": {"$type": "color", "x": {"$value": 1}},
                               "g": {"$type": "dimension", "t": {"$value": "{base.x}"}}}"#]);
        assert_eq!(ty(&a, &["g", "t"]).as_deref(), Some("color"));
        assert!(matches!(
            source(&a, &["g", "t"]),
            TypeSource::FromReference(_)
        ));
        assert!(d.diagnostics().is_empty());
    }

    #[test]
    fn alias_chains_and_ref_tokens_follow_through() {
        let (a, _) = run(&[
            r##"{"a": {"$type": "number", "$value": 1}, "b": {"$value": "{a}"}, "c": {"$value": "{b}"},
                               "d": {"$ref": "#/c"}}"##,
        ]);
        for name in ["b", "c", "d"] {
            assert_eq!(ty(&a, &[name]).as_deref(), Some("number"), "{name}");
        }
    }

    #[test]
    fn explicit_type_on_a_referencing_token_is_kept() {
        let (a, _) = run(&[
            r#"{"a": {"$type": "number", "$value": 1}, "b": {"$type": "dimension", "$value": "{a}"}}"#,
        ]);
        assert_eq!(ty(&a, &["b"]).as_deref(), Some("dimension"));
    }

    #[test]
    fn types_come_through_extends_with_local_override() {
        let (a, d) = run(&[r#"{"base": {"$type": "color", "x": {"$value": 1}},
                               "d1": {"$extends": "{base}"},
                               "d2": {"$extends": "{base}", "$type": "dimension", "y": {"$value": 2}}}"#]);
        assert_eq!(ty(&a, &["d1", "x"]).as_deref(), Some("color"));
        // Inherited tokens live under the local group, so its own `$type` is the closest one.
        assert_eq!(ty(&a, &["d2", "x"]).as_deref(), Some("dimension"));
        assert_eq!(ty(&a, &["d2", "y"]).as_deref(), Some("dimension"));
        assert!(d.diagnostics().is_empty());
    }

    #[test]
    fn root_tokens_use_the_group_type() {
        let (a, _) =
            run(&[r#"{"c": {"$type": "color", "$root": {"$value": 1}, "dark": {"$value": 2}}}"#]);
        assert_eq!(ty(&a, &["c", "$root"]).as_deref(), Some("color"));
    }

    #[test]
    fn a_later_layer_can_supply_the_type() {
        let (a, d) = run(&[
            r#"{"g": {"t": {"$value": 1}}}"#,
            r#"{"g": {"$type": "number"}}"#,
        ]);
        assert_eq!(ty(&a, &["g", "t"]).as_deref(), Some("number"));
        assert!(d.diagnostics().is_empty());
    }

    #[test]
    fn an_untyped_token_is_reported_once() {
        let (a, d) = run(&[r#"{"t": {"$value": 1}}"#]);
        assert_eq!(ty(&a, &["t"]), None);
        assert_eq!(codes(&d), [DiagnosticCode::UnresolvedType]);
    }

    #[test]
    fn untyped_targets_are_reported_on_the_target_only() {
        let (a, d) = run(&[r#"{"a": {"$value": 1}, "b": {"$value": "{a}"}}"#]);
        assert_eq!(ty(&a, &["b"]), None);
        assert_eq!(codes(&d), [DiagnosticCode::UnresolvedType]);
        assert!(d.diagnostics()[0].message.contains("#/a"));
    }

    #[test]
    fn broken_references_do_not_add_a_type_error() {
        let (a, d) = run(&[r#"{"g": {"$type": "color", "t": {"$value": "{missing}"}}}"#]);
        assert_eq!(ty(&a, &["g", "t"]), None);
        assert_eq!(codes(&d), [DiagnosticCode::UnresolvedReference]);
    }

    #[test]
    fn a_ref_into_a_group_falls_back_to_the_parent_groups() {
        let (a, _) = run(&[
            r##"{"h": {"i": {"$value": 1}}, "g": {"$type": "color", "t": {"$ref": "#/h"}}}"##,
        ]);
        assert_eq!(ty(&a, &["g", "t"]).as_deref(), Some("color"));
    }

    #[test]
    fn the_type_is_never_guessed_from_the_value() {
        let (a, d) =
            run(&[r##"{"t": {"$value": {"colorSpace": "srgb", "components": [1, 0, 0]}}}"##]);
        assert_eq!(ty(&a, &["t"]), None);
        assert_eq!(codes(&d), [DiagnosticCode::UnresolvedType]);
    }
}
