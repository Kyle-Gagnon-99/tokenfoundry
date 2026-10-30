//! The `build` module is the last pass: it turns the analyzed IR into the final [`TokenArena`].
//!
//! For every group and token in the lowered tree it
//! 1. assigns a [`TokenId`] (so aliases can name their target),
//! 2. copies the common properties (`$description`, `$deprecated`),
//! 3. takes the token's type from the [`TypeMap`](crate::ir::types::TypeMap) and turns the name into a
//!    [`TokenType`],
//! 4. reads the value in whichever form the author used: a curly alias, a `$ref`, or a literal parsed by the
//!    per-type parser.
//!
//! A token that cannot be built becomes [`AnyToken::Invalid`] so ids stay stable and groups keep their shape. Its
//! reason has been reported, either here or by an earlier pass, so later tooling can show partial results but a
//! backend should refuse to emit when the diagnostics hold errors.

use indexmap::{IndexMap, IndexSet};

use crate::{
    ast::{
        node::{Node, PropId},
        token::{
            AnyToken, Deprecation, GroupToken, RefAliasOrValue, Token, TokenArena, TokenCommon,
            TokenId,
        },
        types::{TokenType, parse_token_value},
        value::{SlotKind, checked_reference, classify_slot, node},
    },
    diagnostics::{
        code::DiagnosticCode,
        diagnostic::{Diagnostic, DiagnosticSink, DiagnosticStore},
    },
    ir::{
        lower::{CommonProps, PropKind, TokenBody},
        node::{DtcgValueSyntax, NodeId},
        pipeline::Analyzed,
        refs::is_alias_like,
        types::{Resolved, TypeSource},
    },
};

pub fn build_tokens(a: &Analyzed, diags: &mut impl DiagnosticSink) -> TokenArena {
    // Parents have lower ids than their children, so ids follow the document order.
    let mut by_prop = IndexMap::new();
    for i in 0..a.lowered.len() as u32 {
        let id = PropId(i);
        if matches!(
            a.lowered.get(id).kind,
            PropKind::Group(_) | PropKind::Token(_)
        ) {
            by_prop.insert(id, TokenId(by_prop.len()));
        }
    }

    let mut builder = Builder {
        a,
        by_prop: &by_prop,
        reported_types: IndexSet::new(),
    };
    let tokens = by_prop
        .keys()
        .map(|&prop| builder.build(prop, diags))
        .collect();

    TokenArena {
        tokens,
        root: by_prop[&a.lowered.root],
        by_prop,
    }
}

struct Builder<'a> {
    a: &'a Analyzed,
    by_prop: &'a IndexMap<PropId, TokenId>,
    /// An unknown `$type` is reported once, even though every token below it sees it.
    reported_types: IndexSet<NodeId>,
}

impl Builder<'_> {
    fn build(&mut self, prop: PropId, diags: &mut impl DiagnosticSink) -> AnyToken {
        let a = self.a;
        let invalid = || AnyToken::Invalid(node(a, prop, ()));

        match &a.lowered.get(prop).kind {
            PropKind::Group(group) => {
                let tokens = group
                    .members
                    .iter()
                    .filter_map(|(name, member)| Some((name.clone(), *self.by_prop.get(member)?)))
                    .collect();
                AnyToken::Group(node(
                    a,
                    prop,
                    GroupToken {
                        common: self.common(group.common),
                        tokens,
                    },
                ))
            }
            PropKind::Token(data) => match self.token(prop, data.common, &data.body, diags) {
                Some(token) => AnyToken::Token(node(a, prop, token)),
                None => invalid(),
            },
            PropKind::Value(_) => invalid(),
        }
    }

    fn common(&self, c: CommonProps) -> TokenCommon {
        let a = self.a;
        TokenCommon {
            description: c
                .description
                .and_then(|p| Some(node(a, p, a.string(p)?.to_string()))),
            deprecation: c.deprecated.and_then(|p| match a.syntax(p) {
                DtcgValueSyntax::Bool(b) => Some(node(a, p, Deprecation::Boolean(*b))),
                DtcgValueSyntax::String(s) => Some(node(a, p, Deprecation::WithMessage(s.clone()))),
                _ => None,
            }),
            extensions: c.extensions,
        }
    }

    fn token(
        &mut self,
        prop: PropId,
        common: CommonProps,
        body: &TokenBody,
        diags: &mut impl DiagnosticSink,
    ) -> Option<Token> {
        let token_type = self.token_type(prop, diags)?;
        let a = self.a;

        let value = match body {
            TokenBody::Invalid => return None,
            // A token-level `$ref`. The token itself is the holder.
            TokenBody::Ref { prop: slot, .. } => {
                let target = a.target(prop)?;
                self.reference(*slot, target, token_type, diags)?
            }
            TokenBody::Value(v) => match classify_slot(a, *v, diags) {
                SlotKind::Broken => return None,
                SlotKind::Reference(target) => self.reference(*v, target, token_type, diags)?,
                SlotKind::Literal => match a.string(*v) {
                    Some(s) if is_alias_like(s) => {
                        // An alias that did not resolve was reported by the reference pass.
                        let target_prop = a.target(*v)?;
                        let target = self.by_prop.get(&target_prop)?;
                        if !self.alias_type_matches(*v, target_prop, token_type, diags) {
                            return None;
                        }
                        node(a, *v, RefAliasOrValue::Alias(*target))
                    }
                    _ => {
                        let parsed = parse_token_value(token_type, *v, a, diags)?;
                        node(a, *v, RefAliasOrValue::Value(parsed))
                    }
                },
            },
        };

        Some(Token {
            common: self.common(common),
            token_type,
            value,
        })
    }

    /// An alias stands for the whole target token, so the target must have the same type. A target whose type is
    /// unknown was reported by the type pass, so it is not reported again.
    fn alias_type_matches(
        &self,
        slot: PropId,
        target: PropId,
        ty: TokenType,
        diags: &mut impl DiagnosticSink,
    ) -> bool {
        let a = self.a;
        let Some(Resolved::Known { name, .. }) = a.types.get(target) else {
            return true;
        };
        if name == ty.as_str() {
            return true;
        }
        diags.emit(
            Diagnostic::new(DiagnosticCode::InvalidTokenValue)
                .with_message(format!(
                    "this alias leads to a `{name}` token, but this token is a `{}`",
                    ty.as_str()
                ))
                .primary(a.span(slot), format!("expected a `{}` token", ty.as_str()))
                .secondary(a.span(target), format!("this token is a `{name}`"))
                .help("an alias takes the whole value of another token, so both must have the same type"),
        );
        false
    }

    /// A reference has to end at something that is a valid value of the token's type.
    fn reference(
        &self,
        slot: PropId,
        target: PropId,
        ty: TokenType,
        diags: &mut impl DiagnosticSink,
    ) -> Option<Node<RefAliasOrValue<crate::ast::types::TokenValue>>> {
        let a = self.a;
        let expected = format!("a valid `{}` value", ty.as_str());
        let end = checked_reference(
            a,
            slot,
            target,
            &expected,
            |p| parse_token_value(ty, p, a, &mut DiagnosticStore::new()).is_some(),
            diags,
        )?;
        Some(node(a, slot, RefAliasOrValue::Reference(end)))
    }

    fn token_type(&mut self, token: PropId, diags: &mut impl DiagnosticSink) -> Option<TokenType> {
        let a = self.a;
        // `Unresolvable` has been reported by the type pass.
        let Some(Resolved::Known { name, source }) = a.types.get(token) else {
            return None;
        };

        if let Some(ty) = TokenType::from_str(name) {
            return Some(ty);
        }

        // A type taken from a referenced token is that token's to report.
        if let TypeSource::Explicit(p) | TypeSource::FromGroup(p) = *source {
            if self.reported_types.insert(a.lowered.get(p).origin) {
                diags.emit(
                    Diagnostic::new(DiagnosticCode::UnknownTokenType)
                        .with_message(format!("`{name}` is not a DTCG type"))
                        .primary(a.span(p), "unknown type")
                        .help(
                            "valid types are color, dimension, fontFamily, fontWeight, duration, cubicBezier, \
                             number, strokeStyle, border, transition, shadow, gradient and typography",
                        ),
                );
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ast::{types::TokenValue, value::ValueOrRef},
        ir::{
            merge::LayerOrigin,
            pipeline::{Layer, compile},
        },
        source::SourceIdGenerator,
    };

    fn build(docs: &[&str]) -> (TokenArena, DiagnosticStore) {
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
            .map(|(source, tree)| Layer {
                source: *source,
                origin: LayerOrigin::File(*source),
                tree,
            })
            .collect();
        let mut diags = DiagnosticStore::new();
        let arena = compile(&layers, &mut diags)
            .ok()
            .expect("not blocked")
            .arena;
        (arena, diags)
    }

    fn token<'a>(arena: &'a TokenArena, path: &[&str]) -> &'a Token {
        match arena.get(arena.find(path).expect("exists")) {
            AnyToken::Token(t) => &t.value,
            other => panic!("not a token: {other:?}"),
        }
    }

    const SPACING: &str = r#"{
        "$type": "dimension",
        "space": {
            "sm": {"$value": {"value": 4, "unit": "px"}, "$description": "small", "$deprecated": "use md"},
            "md": {"$value": "{space.sm}"}
        }
    }"#;

    #[test]
    fn builds_groups_tokens_and_aliases() {
        let (arena, diags) = build(&[SPACING]);
        assert!(!diags.has_errors(), "{:?}", diags.diagnostics());

        let sm = token(&arena, &["space", "sm"]);
        assert_eq!(sm.token_type, TokenType::Dimension);
        assert!(matches!(
            sm.value.value,
            RefAliasOrValue::Value(TokenValue::Dimension(_))
        ));
        assert_eq!(sm.common.description.as_ref().unwrap().value, "small");
        assert!(matches!(
            sm.common.deprecation.as_ref().unwrap().value,
            Deprecation::WithMessage(_)
        ));

        let md = token(&arena, &["space", "md"]);
        let sm_id = arena.find(&["space", "sm"]).unwrap();
        assert!(matches!(md.value.value, RefAliasOrValue::Alias(id) if id == sm_id));
    }

    #[test]
    fn a_later_layer_overrides_a_value() {
        let (arena, diags) = build(&[
            SPACING,
            r#"{"space": {"sm": {"$value": {"value": 8, "unit": "rem"}}}}"#,
        ]);
        assert!(!diags.has_errors());
        let RefAliasOrValue::Value(TokenValue::Dimension(d)) =
            &token(&arena, &["space", "sm"]).value.value
        else {
            panic!()
        };
        assert_eq!(format!("{d:?}").contains("Rem"), true);
        assert_eq!(d.value.value, ValueOrRef::Value(8_f64));
    }

    #[test]
    fn a_ref_slot_becomes_a_reference() {
        let (arena, diags) = build(&[r##"{
            "$type": "dimension",
            "a": {"$value": {"value": 1, "unit": "px"}},
            "b": {"$value": {"value": {"$ref": "#/a/$value/value"}, "unit": "px"}}
        }"##]);
        assert!(!diags.has_errors(), "{:?}", diags.diagnostics());
        assert!(matches!(
            token(&arena, &["b"]).value.value,
            RefAliasOrValue::Value(TokenValue::Dimension(_))
        ));
    }

    #[test]
    fn a_token_level_ref_becomes_a_reference() {
        let (arena, diags) = build(&[r##"{
            "$type": "dimension",
            "a": {"$value": {"value": 1, "unit": "px"}},
            "b": {"$ref": "#/a/$value"}
        }"##]);
        assert!(!diags.has_errors(), "{:?}", diags.diagnostics());
        assert!(matches!(
            token(&arena, &["b"]).value.value,
            RefAliasOrValue::Reference(_)
        ));
    }

    #[test]
    fn a_bad_value_becomes_an_invalid_token_and_keeps_the_rest() {
        let (arena, diags) = build(&[r#"{
            "$type": "dimension",
            "bad": {"$value": {"value": "x", "unit": "px"}},
            "good": {"$value": {"value": 1, "unit": "px"}}
        }"#]);
        assert!(diags.has_errors());
        assert!(matches!(
            arena.get(arena.find(&["bad"]).unwrap()),
            AnyToken::Invalid(_)
        ));
        assert!(matches!(
            arena.get(arena.find(&["good"]).unwrap()),
            AnyToken::Token(_)
        ));
    }

    #[test]
    fn an_unknown_type_is_reported_once_per_declaration() {
        let (arena, diags) = build(&[r#"{
            "$type": "nope",
            "a": {"$value": 1},
            "b": {"$value": 2}
        }"#]);
        let n = diags
            .diagnostics()
            .iter()
            .filter(|d| d.code == DiagnosticCode::UnknownTokenType)
            .count();
        assert_eq!(n, 1);
        assert!(matches!(
            arena.get(arena.find(&["a"]).unwrap()),
            AnyToken::Invalid(_)
        ));
    }

    #[test]
    fn extended_tokens_are_built_under_the_extending_group() {
        let (arena, diags) = build(&[r#"{
            "$type": "dimension",
            "base": {"x": {"$value": {"value": 1, "unit": "px"}}},
            "derived": {"$extends": "{base}"}
        }"#]);
        assert!(!diags.has_errors(), "{:?}", diags.diagnostics());
        token(&arena, &["derived", "x"]);
    }

    #[test]
    fn number_tokens_build_as_values_aliases_and_references() {
        let (arena, diags) = build(&[r##"{
            "$type": "number",
            "a": {"$value": 2},
            "b": {"$value": "{a}"},
            "c": {"$ref": "#/a/$value"}
        }"##]);
        assert!(!diags.has_errors(), "{:?}", diags.diagnostics());

        let a = token(&arena, &["a"]);
        assert_eq!(a.token_type, TokenType::Number);
        assert!(matches!(
            &a.value.value,
            RefAliasOrValue::Value(TokenValue::Number(n)) if n.0 == 2.0
        ));
        let a_id = arena.find(&["a"]).unwrap();
        assert!(
            matches!(token(&arena, &["b"]).value.value, RefAliasOrValue::Alias(id) if id == a_id)
        );
        assert!(matches!(
            token(&arena, &["c"]).value.value,
            RefAliasOrValue::Reference(_)
        ));
    }

    #[test]
    fn a_non_number_value_makes_an_invalid_number_token() {
        let (arena, diags) = build(&[r#"{
            "$type": "number",
            "bad": {"$value": "16"},
            "good": {"$value": 16}
        }"#]);
        assert!(diags.has_errors());
        assert!(matches!(
            arena.get(arena.find(&["bad"]).unwrap()),
            AnyToken::Invalid(_)
        ));
        assert!(matches!(
            arena.get(arena.find(&["good"]).unwrap()),
            AnyToken::Token(_)
        ));
    }

    #[test]
    fn an_alias_to_a_token_of_another_type_is_reported() {
        let (arena, diags) = build(&[r#"{
            "d": {"$type": "dimension", "$value": {"value": 1, "unit": "px"}},
            "n": {"$type": "number", "$value": "{d}"},
            "ok": {"$type": "dimension", "$value": "{d}"}
        }"#]);
        let errors: Vec<_> = diags
            .diagnostics()
            .iter()
            .filter(|d| d.code == DiagnosticCode::InvalidTokenValue)
            .collect();
        assert_eq!(errors.len(), 1, "{:?}", diags.diagnostics());
        assert_eq!(errors[0].labels.len(), 2);
        assert!(matches!(
            arena.get(arena.find(&["n"]).unwrap()),
            AnyToken::Invalid(_)
        ));
        assert!(matches!(
            arena.get(arena.find(&["ok"]).unwrap()),
            AnyToken::Token(_)
        ));
    }

    #[test]
    fn an_alias_inherits_the_type_when_it_has_none() {
        let (arena, diags) = build(&[r#"{
            "d": {"$type": "dimension", "$value": {"value": 1, "unit": "px"}},
            "e": {"$value": "{d}"}
        }"#]);
        assert!(!diags.has_errors(), "{:?}", diags.diagnostics());
        assert_eq!(token(&arena, &["e"]).token_type, TokenType::Dimension);
    }

    #[test]
    fn duration_tokens_build_as_values_aliases_and_references() {
        let (arena, diags) = build(&[r##"{
            "$type": "duration",
            "a": {"$value": {"value": 1, "unit": "s"}},
            "b": {"$value": "{a}"},
            "c": {"$ref": "#/a/$value"}
        }"##]);
        assert!(!diags.has_errors(), "{:?}", diags.diagnostics());

        let a = token(&arena, &["a"]);
        assert_eq!(a.token_type, TokenType::Duration);
        assert!(matches!(
            a.value.value,
            RefAliasOrValue::Value(TokenValue::Duration(_))
        ));
        let a_id = arena.find(&["a"]).unwrap();
        assert!(
            matches!(token(&arena, &["b"]).value.value, RefAliasOrValue::Alias(id) if id == a_id)
        );
        assert!(matches!(
            token(&arena, &["c"]).value.value,
            RefAliasOrValue::Reference(_)
        ));
    }

    #[test]
    fn a_bad_duration_becomes_an_invalid_token_and_keeps_the_rest() {
        let (arena, diags) = build(&[r#"{
            "$type": "duration",
            "bad": {"$value": {"value": 1, "unit": "min"}},
            "good": {"$value": {"value": 1, "unit": "ms"}}
        }"#]);
        assert!(diags.has_errors());
        assert!(matches!(
            arena.get(arena.find(&["bad"]).unwrap()),
            AnyToken::Invalid(_)
        ));
        assert!(matches!(
            arena.get(arena.find(&["good"]).unwrap()),
            AnyToken::Token(_)
        ));
    }

    #[test]
    fn an_alias_between_duration_and_dimension_is_reported() {
        let (_, diags) = build(&[r#"{
            "d": {"$type": "dimension", "$value": {"value": 1, "unit": "px"}},
            "t": {"$type": "duration", "$value": "{d}"}
        }"#]);
        assert!(
            diags
                .diagnostics()
                .iter()
                .any(|d| d.code == DiagnosticCode::InvalidTokenValue)
        );
    }

    const SHADOW: &str = r##"{
        "color": {"colorSpace": "srgb", "components": [0, 0, 0], "alpha": 0.5},
        "offsetX": {"value": 0, "unit": "px"}, "offsetY": {"value": 2, "unit": "px"},
        "blur": {"value": 4, "unit": "px"}, "spread": {"value": 0, "unit": "px"}
    }"##;

    #[test]
    fn shadow_tokens_build_as_values_aliases_and_references() {
        let (arena, diags) = build(&[&format!(
            r##"{{
            "$type": "shadow",
            "a": {{"$value": {SHADOW}}},
            "b": {{"$value": "{{a}}"}},
            "c": {{"$ref": "#/a/$value"}},
            "d": {{"$value": [{SHADOW}, "{{a}}"]}}
        }}"##
        )]);
        assert!(!diags.has_errors(), "{:?}", diags.diagnostics());

        let a = token(&arena, &["a"]);
        assert_eq!(a.token_type, TokenType::Shadow);
        assert!(matches!(
            a.value.value,
            RefAliasOrValue::Value(TokenValue::Shadow(_))
        ));
        let a_id = arena.find(&["a"]).unwrap();
        assert!(
            matches!(token(&arena, &["b"]).value.value, RefAliasOrValue::Alias(id) if id == a_id)
        );
        assert!(matches!(
            token(&arena, &["c"]).value.value,
            RefAliasOrValue::Reference(_)
        ));
        let RefAliasOrValue::Value(TokenValue::Shadow(list)) = &token(&arena, &["d"]).value.value
        else {
            panic!()
        };
        assert_eq!(list.layers.len(), 2);
    }

    #[test]
    fn a_bad_shadow_becomes_an_invalid_token_and_keeps_the_rest() {
        let (arena, diags) = build(&[&format!(
            r#"{{
            "$type": "shadow",
            "bad": {{"$value": {{"color": {{"colorSpace": "srgb", "components": [0, 0, 0]}}}}}},
            "good": {{"$value": {SHADOW}}}
        }}"#
        )]);
        assert!(diags.has_errors());
        assert!(matches!(
            arena.get(arena.find(&["bad"]).unwrap()),
            AnyToken::Invalid(_)
        ));
        assert!(matches!(
            arena.get(arena.find(&["good"]).unwrap()),
            AnyToken::Token(_)
        ));
    }

    #[test]
    fn a_member_alias_to_a_token_of_the_wrong_type_is_reported() {
        let (arena, diags) = build(&[&format!(
            r#"{{
            "d": {{"$type": "dimension", "$value": {{"value": 1, "unit": "px"}}}},
            "s": {{"$type": "shadow", "$value": {}}}
        }}"#,
            SHADOW.replace(
                r#""color": {"colorSpace": "srgb", "components": [0, 0, 0], "alpha": 0.5}"#,
                r#""color": "{d}""#
            )
        )]);
        assert!(diags.has_errors());
        assert!(matches!(
            arena.get(arena.find(&["s"]).unwrap()),
            AnyToken::Invalid(_)
        ));
    }

    #[test]
    fn an_alias_between_shadow_and_color_tokens_is_reported() {
        let (_, diags) = build(&[&format!(
            r#"{{
            "c": {{"$type": "color", "$value": {{"colorSpace": "srgb", "components": [0, 0, 0]}}}},
            "s": {{"$type": "shadow", "$value": "{{c}}"}}
        }}"#
        )]);
        assert!(
            diags
                .diagnostics()
                .iter()
                .any(|d| d.code == DiagnosticCode::InvalidTokenValue)
        );
    }

    #[test]
    fn cubic_bezier_tokens_build_as_values_aliases_and_references() {
        let (arena, diags) = build(&[r##"{
            "$type": "cubicBezier",
            "a": {"$value": [0.1, 0.2, 0.3, 0.4]},
            "b": {"$value": "{a}"},
            "c": {"$ref": "#/a/$value"}
        }"##]);
        assert!(!diags.has_errors(), "{:?}", diags.diagnostics());
        assert_eq!(token(&arena, &["a"]).token_type, TokenType::CubicBezier);
        assert!(matches!(
            token(&arena, &["a"]).value.value,
            RefAliasOrValue::Value(TokenValue::CubicBezier(_))
        ));
        let a_id = arena.find(&["a"]).unwrap();
        assert!(
            matches!(token(&arena, &["b"]).value.value, RefAliasOrValue::Alias(id) if id == a_id)
        );
        assert!(matches!(
            token(&arena, &["c"]).value.value,
            RefAliasOrValue::Reference(_)
        ));
    }

    #[test]
    fn a_bad_cubic_bezier_becomes_an_invalid_token_and_keeps_the_rest() {
        let (arena, diags) = build(&[r#"{
            "$type": "cubicBezier",
            "bad": {"$value": [2, 0, 0.5, 1]},
            "good": {"$value": [0, 0, 1, 1]}
        }"#]);
        assert!(diags.has_errors());
        assert!(matches!(
            arena.get(arena.find(&["bad"]).unwrap()),
            AnyToken::Invalid(_)
        ));
        assert!(matches!(
            arena.get(arena.find(&["good"]).unwrap()),
            AnyToken::Token(_)
        ));
    }

    #[test]
    fn the_remaining_types_build_as_values_aliases_and_references() {
        let (arena, diags) = build(&[r##"{
            "family": {"$type": "fontFamily", "$value": "Inter"},
            "familyAlias": {"$value": "{family}"},
            "weight": {"$type": "fontWeight", "$value": "bold"},
            "stroke": {"$type": "strokeStyle", "$value": "solid"},
            "border": {"$type": "border", "$value": {
                "color": {"colorSpace": "srgb", "components": [0, 0, 0]},
                "width": {"value": 1, "unit": "px"}, "style": "solid"
            }},
            "transition": {"$type": "transition", "$value": {
                "duration": {"value": 1, "unit": "s"}, "delay": {"value": 0, "unit": "s"},
                "timingFunction": [0, 0, 1, 1]
            }},
            "gradient": {"$type": "gradient", "$value": [
                {"color": {"colorSpace": "srgb", "components": [0, 0, 0]}, "position": 0}
            ]},
            "gradientRef": {"$type": "gradient", "$ref": "#/gradient/$value"},
            "type": {"$type": "typography", "$value": {
                "fontFamily": "{family}", "fontSize": {"value": 1, "unit": "rem"},
                "fontWeight": "{weight}", "letterSpacing": {"value": 0, "unit": "px"}, "lineHeight": 1.2
            }}
        }"##]);
        assert!(!diags.has_errors(), "{:?}", diags.diagnostics());
        for (name, ty) in [
            ("family", TokenType::FontFamily),
            ("weight", TokenType::FontWeight),
            ("stroke", TokenType::StrokeStyle),
            ("border", TokenType::Border),
            ("transition", TokenType::Transition),
            ("gradient", TokenType::Gradient),
            ("type", TokenType::Typography),
        ] {
            let t = token(&arena, &[name]);
            assert_eq!(t.token_type, ty, "{name}");
            assert!(matches!(t.value.value, RefAliasOrValue::Value(_)), "{name}");
        }
        let family = arena.find(&["family"]).unwrap();
        assert!(
            matches!(token(&arena, &["familyAlias"]).value.value, RefAliasOrValue::Alias(id) if id == family)
        );
        assert_eq!(
            token(&arena, &["familyAlias"]).token_type,
            TokenType::FontFamily
        );
        assert!(matches!(
            token(&arena, &["gradientRef"]).value.value,
            RefAliasOrValue::Reference(_)
        ));
    }

    #[test]
    fn bad_values_of_the_remaining_types_become_invalid_tokens_and_keep_the_rest() {
        let (arena, diags) = build(&[r#"{
            "weight": {"$type": "fontWeight", "$value": 5000},
            "stroke": {"$type": "strokeStyle", "$value": "wavy"},
            "family": {"$type": "fontFamily", "$value": []},
            "gradient": {"$type": "gradient", "$value": []},
            "border": {"$type": "border", "$value": {}},
            "transition": {"$type": "transition", "$value": {}},
            "type": {"$type": "typography", "$value": {}},
            "good": {"$type": "fontWeight", "$value": 400}
        }"#]);
        assert!(diags.has_errors());
        for name in [
            "weight",
            "stroke",
            "family",
            "gradient",
            "border",
            "transition",
            "type",
        ] {
            assert!(
                matches!(
                    arena.get(arena.find(&[name]).unwrap()),
                    AnyToken::Invalid(_)
                ),
                "{name}"
            );
        }
        assert!(matches!(
            arena.get(arena.find(&["good"]).unwrap()),
            AnyToken::Token(_)
        ));
    }

    #[test]
    fn an_alias_between_border_and_typography_tokens_is_reported() {
        let (_, diags) = build(&[r#"{
            "t": {"$type": "typography", "$value": {
                "fontFamily": "Inter", "fontSize": {"value": 1, "unit": "px"}, "fontWeight": 400,
                "letterSpacing": {"value": 0, "unit": "px"}, "lineHeight": 1
            }},
            "b": {"$type": "border", "$value": "{t}"}
        }"#]);
        assert!(diags.has_errors());
    }
}
