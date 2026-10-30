//! The `shadow` module provides the AST representation of shadow types in DTCG.
//!
//! A shadow is an object with `color`, `offsetX`, `offsetY`, `blur` and `spread`, and an optional `inset` that
//! defaults to false. It can also be a non-empty array of shadows, which layers them. Each array item is a shadow
//! object, or a reference to another shadow token.
//!
//! `color` and the four dimensions are whole values of their own type, so each can be a literal, a `$ref` that
//! leads to one, or a `{token}` alias to a token of that type. `inset` is a boolean or a `$ref`.
//!
//! Both forms are read as a list of layers, so a lone object is a list of one.

use crate::{
    ast::{
        node::{Node, PropId},
        types::{FromProp, TokenType, color::Color, dimension::Dimension},
        value::{
            SlotKind, ValueOrRef, ValueRefOrAlias, alias_target, bool_slot, checked_reference,
            classify_slot, composite_slot, node,
        },
    },
    diagnostics::{
        code::DiagnosticCode,
        diagnostic::{Diagnostic, DiagnosticSink, DiagnosticStore},
    },
    ir::{pipeline::Analyzed, refs::is_alias_like},
};

#[derive(Debug, Clone, PartialEq)]
pub struct ShadowLayer {
    pub color: Node<ValueRefOrAlias<Color>>,
    pub offset_x: Node<ValueRefOrAlias<Dimension>>,
    pub offset_y: Node<ValueRefOrAlias<Dimension>>,
    pub blur: Node<ValueRefOrAlias<Dimension>>,
    pub spread: Node<ValueRefOrAlias<Dimension>>,
    /// `None` when omitted, which means a drop shadow.
    pub inset: Option<Node<ValueOrRef<bool>>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Shadow {
    /// In the order written. A `Reference` or `Alias` entry leads to a whole shadow, which can itself be a list.
    pub layers: Vec<Node<ValueRefOrAlias<ShadowLayer>>>,
}

impl FromProp for Shadow {
    fn from_prop(prop: PropId, a: &Analyzed, diags: &mut impl DiagnosticSink) -> Option<Self> {
        let Some(items) = a.items(prop) else {
            let layer = ShadowLayer::from_prop(prop, a, diags)?;
            return Some(Shadow {
                layers: vec![node(a, prop, ValueRefOrAlias::Value(layer))],
            });
        };

        if items.is_empty() {
            diags.emit(
                Diagnostic::new(DiagnosticCode::InvalidTokenValue)
                    .with_message("a shadow list must have at least one shadow")
                    .primary(a.span(prop), "add a shadow, or use a single shadow object"),
            );
            return None;
        }

        // Every item is read, so one run reports every problem in the list.
        let layers: Vec<_> = items.iter().map(|&item| entry(a, item, diags)).collect();
        Some(Shadow {
            layers: layers.into_iter().collect::<Option<_>>()?,
        })
    }
}

/// One item of a shadow list: a shadow object, or a reference to a shadow token.
fn entry(
    a: &Analyzed,
    item: PropId,
    diags: &mut impl DiagnosticSink,
) -> Option<Node<ValueRefOrAlias<ShadowLayer>>> {
    match classify_slot(a, item, diags) {
        SlotKind::Broken => None,
        SlotKind::Reference(target) => {
            // A reference can lead to a single shadow or to a list.
            let end = checked_reference(
                a,
                item,
                target,
                "a valid `shadow` value",
                |p| Shadow::from_prop(p, a, &mut DiagnosticStore::new()).is_some(),
                diags,
            )?;
            Some(node(a, item, ValueRefOrAlias::Reference(end)))
        }
        SlotKind::Literal => match a.string(item) {
            Some(s) if is_alias_like(s) => {
                let target = alias_target(a, item, TokenType::Shadow, diags)?;
                Some(node(a, item, ValueRefOrAlias::Alias(target)))
            }
            _ => {
                let layer = ShadowLayer::from_prop(item, a, diags)?;
                Some(node(a, item, ValueRefOrAlias::Value(layer)))
            }
        },
    }
}

impl ShadowLayer {
    const KEYS: [&'static str; 6] = ["color", "offsetX", "offsetY", "blur", "spread", "inset"];
}

impl FromProp for ShadowLayer {
    fn from_prop(prop: PropId, a: &Analyzed, diags: &mut impl DiagnosticSink) -> Option<Self> {
        let Some(members) = a.members(prop) else {
            diags.emit(
                Diagnostic::new(DiagnosticCode::InvalidPropertyShape)
                    .with_message("a shadow must be an object or a list of shadows")
                    .primary(
                        a.span(prop),
                        "expected {\"color\": ..., \"offsetX\": ..., \"offsetY\": ..., \"blur\": ..., \"spread\": ...}",
                    ),
            );
            return None;
        };

        // The schema forbids other members. This is reported but does not stop the known ones from being read.
        for (key, &member) in members {
            if !Self::KEYS.contains(&key.as_str()) {
                diags.emit(
                    Diagnostic::new(DiagnosticCode::InvalidPropertyShape)
                        .with_message(format!("unknown shadow property `{key}`"))
                        .primary(
                            a.span(member),
                            "a shadow only has `color`, `offsetX`, `offsetY`, `blur`, `spread` and `inset`",
                        ),
                );
            }
        }

        let color = field::<Color>(a, prop, "color", TokenType::Color, diags);
        let offset_x = field::<Dimension>(a, prop, "offsetX", TokenType::Dimension, diags);
        let offset_y = field::<Dimension>(a, prop, "offsetY", TokenType::Dimension, diags);
        let blur = field::<Dimension>(a, prop, "blur", TokenType::Dimension, diags);
        let spread = field::<Dimension>(a, prop, "spread", TokenType::Dimension, diags);
        let inset = a
            .member(prop, "inset")
            .map(|slot| bool_slot(a, slot, "a shadow `inset`", diags));
        let inset = match inset {
            Some(inset) => Some(inset?),
            None => None,
        };

        Some(ShadowLayer {
            color: color?,
            offset_x: offset_x?,
            offset_y: offset_y?,
            blur: blur?,
            spread: spread?,
            inset,
        })
    }
}

fn field<T: FromProp>(
    a: &Analyzed,
    prop: PropId,
    key: &str,
    ty: TokenType,
    diags: &mut impl DiagnosticSink,
) -> Option<Node<ValueRefOrAlias<T>>> {
    match a.member(prop, key) {
        Some(slot) => composite_slot(a, slot, ty, diags),
        None => {
            diags.emit(
                Diagnostic::new(DiagnosticCode::InvalidPropertyShape)
                    .with_message(format!("a shadow is missing `{key}`"))
                    .primary(a.span(prop), format!("add `{key}` here"))
                    .help("`color`, `offsetX`, `offsetY`, `blur` and `spread` are all required"),
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        diagnostics::diagnostic::DiagnosticStore,
        ir::{
            lower::{PropKind, TokenBody},
            merge::LayerOrigin,
            pipeline::{Layer, analyze},
        },
        source::SourceIdGenerator,
    };

    const RED: &str = r#"{"colorSpace": "srgb", "components": [1, 0, 0]}"#;
    const PX: &str = r#"{"value": 2, "unit": "px"}"#;

    fn layer_json(extra: &str) -> String {
        format!(
            r#"{{"color": {RED}, "offsetX": {PX}, "offsetY": {PX}, "blur": {PX}, "spread": {PX}{extra}}}"#
        )
    }

    /// Analyzes `{"t": {"$type": "shadow", "$value": <value>}}` plus `extra` top-level members, and parses the
    /// `$value`.
    fn parse(value: &str, extra: &str) -> (Option<Shadow>, DiagnosticStore, Analyzed) {
        let doc = format!(r#"{{"t": {{"$type": "shadow", "$value": {value}}}{extra}}}"#);
        let (tree, errors) = json_spanned::parse_json(&doc);
        assert!(errors.is_empty(), "bad test json: {doc}");
        let tree = tree.unwrap();

        let source = SourceIdGenerator::default().generate();
        let layers = [Layer {
            source,
            origin: LayerOrigin::File(source),
            tree: &tree,
        }];
        let analyzed = analyze(&layers, &mut DiagnosticStore::new()).expect("no cycles");

        let token = analyzed.lowered.member(&["t"]).unwrap();
        let PropKind::Token(data) = &analyzed.lowered.get(token).kind else {
            panic!()
        };
        let TokenBody::Value(v) = data.body else {
            panic!("no $value")
        };

        let mut diags = DiagnosticStore::new();
        let result = Shadow::from_prop(v, &analyzed, &mut diags);
        (result, diags, analyzed)
    }

    fn codes(d: &DiagnosticStore) -> Vec<DiagnosticCode> {
        d.diagnostics().iter().map(|d| d.code).collect()
    }

    fn ok(value: &str, extra: &str) -> Shadow {
        let (result, diags, _) = parse(value, extra);
        assert!(diags.diagnostics().is_empty(), "{:?}", diags.diagnostics());
        result.expect("a shadow")
    }

    fn layer(shadow: &Shadow, i: usize) -> &ShadowLayer {
        match &shadow.layers[i].value {
            ValueRefOrAlias::Value(l) => l,
            other => panic!("not a literal layer: {other:?}"),
        }
    }

    #[test]
    fn parses_a_single_shadow_as_a_list_of_one() {
        let shadow = ok(&layer_json(""), "");
        assert_eq!(shadow.layers.len(), 1);
        let l = layer(&shadow, 0);
        assert!(matches!(l.color.value, ValueRefOrAlias::Value(_)));
        assert!(matches!(l.blur.value, ValueRefOrAlias::Value(_)));
        assert!(l.inset.is_none());
    }

    #[test]
    fn inset_is_read_as_a_boolean() {
        for (text, expected) in [("true", true), ("false", false)] {
            let shadow = ok(&layer_json(&format!(r#", "inset": {text}"#)), "");
            assert_eq!(
                layer(&shadow, 0).inset.as_ref().unwrap().value,
                ValueOrRef::Value(expected)
            );
        }
    }

    #[test]
    fn parses_a_list_of_shadows_in_order() {
        let a = layer_json("");
        let b = layer_json(r#", "inset": true"#);
        let shadow = ok(&format!("[{a}, {b}]"), "");
        assert_eq!(shadow.layers.len(), 2);
        assert!(layer(&shadow, 0).inset.is_none());
        assert!(layer(&shadow, 1).inset.is_some());
    }

    #[test]
    fn member_order_does_not_matter() {
        ok(
            &format!(
                r#"{{"spread": {PX}, "blur": {PX}, "offsetY": {PX}, "offsetX": {PX}, "color": {RED}}}"#
            ),
            "",
        );
    }

    #[test]
    fn nodes_keep_their_own_spans() {
        let (result, _, a) = parse(&layer_json(""), "");
        let shadow = result.unwrap();
        let l = layer(&shadow, 0);
        assert_ne!(l.color.span, l.blur.span);
        assert_eq!(l.color.span, a.span(l.color.id));
    }

    #[test]
    fn an_empty_list_is_rejected() {
        let (result, diags, _) = parse("[]", "");
        assert!(result.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidTokenValue]);
    }

    #[test]
    fn a_value_that_is_neither_an_object_nor_a_list_is_rejected() {
        for value in ["1", r#""0 0 4px red""#, "true"] {
            let (result, diags, _) = parse(value, "");
            assert!(result.is_none(), "{value}");
            assert_eq!(
                codes(&diags),
                [DiagnosticCode::InvalidPropertyShape],
                "{value}"
            );
        }
    }

    #[test]
    fn every_missing_member_is_reported() {
        let (result, diags, _) = parse("{}", "");
        assert!(result.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidPropertyShape; 5]);

        // `inset` is the only optional one.
        let (result, diags, _) = parse(&format!(r#"{{"color": {RED}}}"#), "");
        assert!(result.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidPropertyShape; 4]);
    }

    #[test]
    fn unknown_members_are_reported_but_the_value_is_still_read() {
        let (result, diags, _) = parse(&layer_json(r#", "opacity": 0.5"#), "");
        assert!(result.is_some());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidPropertyShape]);
    }

    #[test]
    fn problems_inside_members_are_reported_with_their_own_errors() {
        // A bad color, a bad unit and a non-boolean inset, all at once.
        let (result, diags, _) = parse(
            &format!(
                r#"{{"color": {{"colorSpace": "srgb", "components": [2, 0, 0]}},
                    "offsetX": {{"value": 1, "unit": "em"}}, "offsetY": {PX}, "blur": {PX}, "spread": {PX},
                    "inset": "yes"}}"#
            ),
            "",
        );
        assert!(result.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidTokenValue; 3]);
    }

    #[test]
    fn every_bad_member_of_every_list_item_is_reported() {
        let bad = layer_json("").replace(r#""unit": "px""#, r#""unit": "em""#);
        let (result, diags, _) = parse(&format!("[{bad}, {bad}]"), "");
        assert!(result.is_none());
        // Four dimensions per item, two items.
        assert_eq!(diags.diagnostics().len(), 8);
    }

    #[test]
    fn members_can_be_ref_objects_that_lead_to_a_valid_value() {
        let (result, diags, a) = parse(
            &format!(
                r##"{{"color": {{"$ref": "#/c/$value"}}, "offsetX": {{"$ref": "#/d/$value"}}, "offsetY": {PX},
                    "blur": {PX}, "spread": {PX}, "inset": {{"$ref": "#/other/$value/inset"}}}}"##
            ),
            &format!(
                r##", "c": {{"$type": "color", "$value": {RED}}},
                    "d": {{"$type": "dimension", "$value": {PX}}},
                    "other": {{"$type": "shadow", "$value": {}}}"##,
                layer_json(r#", "inset": true"#)
            ),
        );
        assert!(diags.diagnostics().is_empty(), "{:?}", diags.diagnostics());
        let shadow = result.unwrap();
        let l = layer(&shadow, 0);
        let ValueRefOrAlias::Reference(end) = l.color.value else {
            panic!()
        };
        assert_ne!(l.color.id, end);
        assert!(a.members(end).is_some());
        assert!(matches!(l.offset_x.value, ValueRefOrAlias::Reference(_)));
        assert!(matches!(
            l.inset.as_ref().unwrap().value,
            ValueOrRef::Reference(_)
        ));
    }

    #[test]
    fn a_ref_that_leads_to_the_wrong_kind_of_value_is_rejected_at_both_ends() {
        // offsetX points at a color, and the color points at a dimension.
        let (result, diags, _) = parse(
            &format!(
                r##"{{"color": {{"$ref": "#/d/$value"}}, "offsetX": {{"$ref": "#/c/$value"}}, "offsetY": {PX},
                    "blur": {PX}, "spread": {PX}}}"##
            ),
            &format!(
                r##", "c": {{"$type": "color", "$value": {RED}}},
                    "d": {{"$type": "dimension", "$value": {PX}}}"##
            ),
        );
        assert!(result.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidTokenValue; 2]);
        assert!(diags.diagnostics().iter().all(|d| d.labels.len() == 2));
    }

    #[test]
    fn a_ref_to_one_part_of_a_dimension_is_not_a_dimension() {
        let (result, diags, _) = parse(
            &format!(
                r##"{{"color": {RED}, "offsetX": {{"$ref": "#/d/$value/value"}}, "offsetY": {PX},
                    "blur": {PX}, "spread": {PX}}}"##
            ),
            &format!(r##", "d": {{"$type": "dimension", "$value": {PX}}}"##),
        );
        assert!(result.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidTokenValue]);
    }

    #[test]
    fn a_ref_to_a_token_instead_of_its_value_is_an_error_with_a_fix() {
        let (result, diags, _) = parse(
            &format!(
                r##"{{"color": {{"$ref": "#/c"}}, "offsetX": {PX}, "offsetY": {PX}, "blur": {PX}, "spread": {PX}}}"##
            ),
            &format!(r##", "c": {{"$type": "color", "$value": {RED}}}"##),
        );
        assert!(result.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidTokenValue]);
        assert!(
            diags.diagnostics()[0]
                .help
                .as_ref()
                .unwrap()
                .contains("/$value")
        );
    }

    #[test]
    fn whole_value_members_can_be_aliases_to_tokens_of_that_type() {
        let (result, diags, a) = parse(
            r#"{"color": "{c}", "offsetX": "{d}", "offsetY": "{d}", "blur": "{d}", "spread": "{d}"}"#,
            &format!(
                r##", "c": {{"$type": "color", "$value": {RED}}},
                    "d": {{"$type": "dimension", "$value": {PX}}}"##
            ),
        );
        assert!(diags.diagnostics().is_empty(), "{:?}", diags.diagnostics());
        let shadow = result.unwrap();
        let l = layer(&shadow, 0);
        let ValueRefOrAlias::Alias(target) = l.color.value else {
            panic!()
        };
        // The alias names the whole token, not its value.
        assert!(matches!(a.lowered.get(target).kind, PropKind::Token(_)));
        assert!(matches!(l.spread.value, ValueRefOrAlias::Alias(_)));
    }

    #[test]
    fn an_alias_to_a_token_of_another_type_is_rejected_at_both_ends() {
        let (result, diags, _) = parse(
            &format!(
                r#"{{"color": "{{d}}", "offsetX": {PX}, "offsetY": {PX}, "blur": {PX}, "spread": {PX}}}"#
            ),
            &format!(r##", "d": {{"$type": "dimension", "$value": {PX}}}"##),
        );
        assert!(result.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidTokenValue]);
        assert_eq!(diags.diagnostics()[0].labels.len(), 2);
    }

    #[test]
    fn an_alias_that_did_not_resolve_is_left_to_the_reference_pass() {
        let (result, diags, _) = parse(
            &format!(
                r#"{{"color": "{{nope}}", "offsetX": {PX}, "offsetY": {PX}, "blur": {PX}, "spread": {PX}}}"#
            ),
            "",
        );
        assert!(result.is_none());
        assert!(diags.diagnostics().is_empty());
    }

    #[test]
    fn an_alias_is_not_allowed_for_inset() {
        let (result, diags, _) = parse(&layer_json(r#", "inset": "{flag}""#), "");
        assert!(result.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidTokenValue]);
        assert!(
            diags.diagnostics()[0]
                .help
                .as_ref()
                .unwrap()
                .contains("$ref")
        );
    }

    #[test]
    fn list_items_can_be_aliases_or_refs_to_shadow_tokens() {
        let other = layer_json("");
        let (result, diags, _) = parse(
            r##"[{"$ref": "#/s/$value"}, "{s}", "{l}"]"##,
            &format!(
                r##", "s": {{"$type": "shadow", "$value": {other}}},
                    "l": {{"$type": "shadow", "$value": [{other}, {other}]}}"##
            ),
        );
        assert!(diags.diagnostics().is_empty(), "{:?}", diags.diagnostics());
        let shadow = result.unwrap();
        assert!(matches!(
            shadow.layers[0].value,
            ValueRefOrAlias::Reference(_)
        ));
        assert!(matches!(shadow.layers[1].value, ValueRefOrAlias::Alias(_)));
        assert!(matches!(shadow.layers[2].value, ValueRefOrAlias::Alias(_)));
    }

    #[test]
    fn a_list_item_that_refers_to_something_that_is_not_a_shadow_is_rejected() {
        let (result, diags, _) = parse(
            r##"["{c}", {"$ref": "#/c/$value"}]"##,
            &format!(r##", "c": {{"$type": "color", "$value": {RED}}}"##),
        );
        assert!(result.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidTokenValue; 2]);
    }

    #[test]
    fn a_list_item_that_is_not_an_object_is_rejected() {
        let (result, diags, _) = parse(r#"[1]"#, "");
        assert!(result.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidPropertyShape]);
    }

    #[test]
    fn a_reference_object_with_other_members_is_rejected() {
        let (result, diags, _) = parse(
            &format!(
                r##"{{"color": {{"$ref": "#/x", "extra": 1}}, "offsetX": {PX}, "offsetY": {PX}, "blur": {PX}, "spread": {PX}}}"##
            ),
            "",
        );
        assert!(result.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidPropertyShape]);
    }
}
