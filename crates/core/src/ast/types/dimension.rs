//! The `dimension` module provides the AST representation of dimension types in DTCG.
//!
//! The value is an object with a numeric `value` and a `unit` of `"px"` or `"rem"`. Both are required, even when the
//! value is 0, and no other members are allowed (the spec's 8.2 and its `dimension.json` schema). Each property can
//! be a `{"$ref": ...}` object instead of a literal.

use crate::{
    ast::{
        node::{Node, PropId},
        types::FromProp,
        value::{ValueOrRef, choice_slot, number_slot},
    },
    diagnostics::{
        code::DiagnosticCode,
        diagnostic::{Diagnostic, DiagnosticSink},
    },
    ir::pipeline::Analyzed,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum DimensionUnit {
    Px,
    Rem,
}

impl DimensionUnit {
    const NAMES: [&'static str; 2] = ["px", "rem"];

    pub(crate) fn parse(s: &str) -> Option<Self> {
        match s {
            "px" => Some(Self::Px),
            "rem" => Some(Self::Rem),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Dimension {
    pub value: Node<ValueOrRef<f64>>,
    pub unit: Node<ValueOrRef<DimensionUnit>>,
}

impl FromProp for Dimension {
    fn from_prop(prop: PropId, a: &Analyzed, diags: &mut impl DiagnosticSink) -> Option<Self> {
        let Some(members) = a.members(prop) else {
            diags.emit(
                Diagnostic::new(DiagnosticCode::InvalidPropertyShape)
                    .with_message("a dimension must be an object")
                    .primary(a.span(prop), "expected {\"value\": 16, \"unit\": \"px\"}"),
            );
            return None;
        };

        // The schema forbids other members. This is reported but does not stop the known ones from being read.
        for (key, &member) in members {
            if key != "value" && key != "unit" {
                diags.emit(
                    Diagnostic::new(DiagnosticCode::InvalidPropertyShape)
                        .with_message(format!("unknown dimension property `{key}`"))
                        .primary(a.span(member), "a dimension only has `value` and `unit`"),
                );
            }
        }

        // Both properties are always read, so one run reports every problem in the value.
        let value = match a.member(prop, "value") {
            Some(slot) => number_slot(a, slot, "a dimension `value`", diags),
            None => {
                report_missing(a, prop, "value", diags);
                None
            }
        };
        let unit = match a.member(prop, "unit") {
            Some(slot) => choice_slot(
                a,
                slot,
                "a dimension `unit`",
                &DimensionUnit::NAMES,
                DimensionUnit::parse,
                diags,
            ),
            None => {
                report_missing(a, prop, "unit", diags);
                None
            }
        };

        Some(Dimension {
            value: value?,
            unit: unit?,
        })
    }
}

fn report_missing(a: &Analyzed, prop: PropId, key: &str, diags: &mut impl DiagnosticSink) {
    let mut diag = Diagnostic::new(DiagnosticCode::InvalidPropertyShape)
        .with_message(format!("a dimension is missing `{key}`"))
        .primary(a.span(prop), format!("add `{key}` here"));
    if key == "unit" {
        diag = diag.help("`unit` is required, even when `value` is 0");
    }
    diags.emit(diag);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ast::reference::JsonPointer,
        diagnostics::diagnostic::DiagnosticStore,
        ir::{
            lower::{PropKind, TokenBody},
            merge::LayerOrigin,
            pipeline::{Layer, analyze},
        },
        source::SourceIdGenerator,
    };

    /// Analyzes `{"t": {"$type": "dimension", "$value": <value>}}` plus `extra` top-level members, and parses the
    /// `$value`. Returns the result, the diagnostics from parsing only, and the analysis for further asserts.
    fn parse(value: &str, extra: &str) -> (Option<Dimension>, DiagnosticStore, Analyzed) {
        let doc = format!(r#"{{"t": {{"$type": "dimension", "$value": {value}}}{extra}}}"#);
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
        let result = Dimension::from_prop(v, &analyzed, &mut diags);
        (result, diags, analyzed)
    }

    fn codes(d: &DiagnosticStore) -> Vec<DiagnosticCode> {
        d.diagnostics().iter().map(|d| d.code).collect()
    }

    fn ok(value: &str) -> Dimension {
        let (result, diags, _) = parse(value, "");
        assert!(diags.diagnostics().is_empty(), "{:?}", diags.diagnostics());
        result.expect("a dimension")
    }

    #[test]
    fn parses_px_and_rem() {
        let px = ok(r#"{"value": 16, "unit": "px"}"#);
        assert_eq!(px.value.value, ValueOrRef::Value(16.0));
        assert_eq!(px.unit.value, ValueOrRef::Value(DimensionUnit::Px));

        let rem = ok(r#"{"value": 0.5, "unit": "rem"}"#);
        assert_eq!(rem.value.value, ValueOrRef::Value(0.5));
        assert_eq!(rem.unit.value, ValueOrRef::Value(DimensionUnit::Rem));
    }

    #[test]
    fn zero_negative_and_exponent_values_are_numbers() {
        assert_eq!(
            ok(r#"{"value": 0, "unit": "px"}"#).value.value,
            ValueOrRef::Value(0.0)
        );
        assert_eq!(
            ok(r#"{"value": -4, "unit": "px"}"#).value.value,
            ValueOrRef::Value(-4.0)
        );
        assert_eq!(
            ok(r#"{"value": 1.5e1, "unit": "px"}"#).value.value,
            ValueOrRef::Value(15.0)
        );
    }

    #[test]
    fn member_order_does_not_matter() {
        assert_eq!(
            ok(r#"{"unit": "px", "value": 1}"#).unit.value,
            ValueOrRef::Value(DimensionUnit::Px)
        );
    }

    #[test]
    fn nodes_keep_their_own_spans() {
        let (result, _, a) = parse(r#"{"value": 16, "unit": "px"}"#, "");
        let dim = result.unwrap();

        // Each leaf points at its own text, not the whole object.
        assert_ne!(dim.value.span, dim.unit.span);
        assert_eq!(dim.value.span, a.span(dim.value.id));
        assert_eq!(a.number(dim.value.id), Some(16.0));
        assert_eq!(a.string(dim.unit.id), Some("px"));
    }

    #[test]
    fn a_ref_object_becomes_a_reference_to_the_final_literal() {
        let (result, diags, a) = parse(
            r##"{"value": {"$ref": "#/base/$value"}, "unit": {"$ref": "#/u/$value/unit"}}"##,
            r##", "base": {"$type": "number", "$value": 16}, "u": {"$type": "dimension", "$value": {"value": 1, "unit": "rem"}}"##,
        );
        assert!(diags.diagnostics().is_empty(), "{:?}", diags.diagnostics());
        let dim = result.unwrap();

        let base = a
            .lowered
            .resolve_pointer(&JsonPointer::parse_pointer("/base/$value").unwrap())
            .unwrap();
        assert_eq!(dim.value.value, ValueOrRef::Reference(base));
        // The node still records the slot that held the reference.
        assert_ne!(dim.value.id, base);
        assert!(matches!(dim.unit.value, ValueOrRef::Reference(_)));
    }

    #[test]
    fn chains_of_refs_and_aliases_end_at_the_final_literal() {
        // value -> ref object -> token whose $value is an alias -> a token holding the number.
        let (result, diags, a) = parse(
            r##"{"value": {"$ref": "#/hop/$value"}, "unit": "px"}"##,
            r##", "hop": {"$type": "number", "$value": "{leaf}"}, "leaf": {"$type": "number", "$value": 8}"##,
        );
        assert!(diags.diagnostics().is_empty(), "{:?}", diags.diagnostics());

        let leaf = a
            .lowered
            .resolve_pointer(&JsonPointer::parse_pointer("/leaf/$value").unwrap())
            .unwrap();
        assert_eq!(result.unwrap().value.value, ValueOrRef::Reference(leaf));
    }

    #[test]
    fn a_ref_token_in_the_chain_is_followed() {
        let (result, diags, a) = parse(
            r##"{"value": {"$ref": "#/hop/$value"}, "unit": "px"}"##,
            r##", "hop": {"$type": "number", "$value": "{via}"}, "via": {"$type": "number", "$ref": "#/leaf/$value"}, "leaf": {"$type": "number", "$value": 8}"##,
        );
        assert!(diags.diagnostics().is_empty(), "{:?}", diags.diagnostics());
        let leaf = a
            .lowered
            .resolve_pointer(&JsonPointer::parse_pointer("/leaf/$value").unwrap())
            .unwrap();
        assert_eq!(result.unwrap().value.value, ValueOrRef::Reference(leaf));
    }

    #[test]
    fn a_ref_to_the_wrong_kind_of_literal_is_rejected_at_both_ends() {
        let (result, diags, _) = parse(
            r##"{"value": {"$ref": "#/s/$value"}, "unit": {"$ref": "#/n/$value"}}"##,
            r##", "s": {"$type": "number", "$value": "text"}, "n": {"$type": "number", "$value": 3}"##,
        );
        assert!(result.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidTokenValue; 2]);
        // Primary label on the slot, secondary on where the reference ends.
        assert_eq!(diags.diagnostics()[0].labels.len(), 2);
        assert!(diags.diagnostics()[1].message.contains(r#""px", "rem""#));
    }

    #[test]
    fn a_ref_to_an_object_or_an_unknown_unit_is_rejected() {
        for target in [r#"{"k": 1}"#, r#""em""#, "null"] {
            let (result, diags, _) = parse(
                r##"{"value": 1, "unit": {"$ref": "#/tgt/$value"}}"##,
                &format!(r#", "tgt": {{"$type": "number", "$value": {target}}}"#),
            );
            assert!(result.is_none(), "{target}");
            assert_eq!(
                codes(&diags),
                [DiagnosticCode::InvalidTokenValue],
                "{target}"
            );
        }
    }

    #[test]
    fn a_ref_to_a_token_instead_of_its_value_is_an_error_with_a_fix() {
        let (result, diags, _) = parse(
            r##"{"value": {"$ref": "#/base"}, "unit": "px"}"##,
            r##", "base": {"$type": "number", "$value": 16}"##,
        );
        assert!(result.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidTokenValue]);
        let help = diags.diagnostics()[0].help.as_deref().unwrap();
        assert!(help.contains("#/base/$value"), "{help}");
    }

    #[test]
    fn a_ref_to_a_group_is_an_error() {
        let (result, diags, _) = parse(
            r##"{"value": {"$ref": "#/g"}, "unit": "px"}"##,
            r##", "g": {"inner": {"$type": "number", "$value": 1}}"##,
        );
        assert!(result.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidTokenValue]);
        assert!(diags.diagnostics()[0].message.contains("group"));
    }

    #[test]
    fn a_chain_that_passes_through_a_token_via_ref_is_an_error() {
        // `#/hop` is a token, reached through a `$ref` object rather than an alias.
        let (result, diags, _) = parse(
            r##"{"value": {"$ref": "#/mid/$value"}, "unit": "px"}"##,
            r##", "mid": {"$type": "number", "$value": {"$ref": "#/hop"}}, "hop": {"$type": "number", "$value": 1}"##,
        );
        assert!(result.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidTokenValue]);
    }

    #[test]
    fn a_value_that_is_not_an_object_is_rejected() {
        for bad in [r#"16"#, r#""16px""#, r#"[16, "px"]"#, r#"null"#] {
            let (result, diags, _) = parse(bad, "");
            assert!(result.is_none(), "{bad}");
            assert_eq!(
                codes(&diags),
                [DiagnosticCode::InvalidPropertyShape],
                "{bad}"
            );
        }
    }

    #[test]
    fn missing_members_are_each_reported() {
        let (result, diags, _) = parse(r#"{}"#, "");
        assert!(result.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidPropertyShape; 2]);

        let (_, diags, _) = parse(r#"{"value": 0}"#, "");
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidPropertyShape]);
        assert!(
            diags.diagnostics()[0]
                .help
                .as_deref()
                .unwrap()
                .contains("even when `value` is 0")
        );
    }

    #[test]
    fn a_bad_unit_lists_the_choices() {
        for bad in [r#""em""#, r#""PX""#, r#""""#, r#"16"#, r#"null"#] {
            let (result, diags, _) = parse(&format!(r#"{{"value": 1, "unit": {bad}}}"#), "");
            assert!(result.is_none(), "{bad}");
            assert_eq!(codes(&diags), [DiagnosticCode::InvalidTokenValue], "{bad}");
            assert!(diags.diagnostics()[0].message.contains(r#""px", "rem""#));
        }
    }

    #[test]
    fn a_bad_value_is_rejected() {
        for bad in [r#""16""#, r#"true"#, r#"null"#, r#"[1]"#, r#"{}"#] {
            let (result, diags, _) = parse(&format!(r#"{{"value": {bad}, "unit": "px"}}"#), "");
            assert!(result.is_none(), "{bad}");
            assert_eq!(codes(&diags), [DiagnosticCode::InvalidTokenValue], "{bad}");
        }
    }

    #[test]
    fn every_problem_is_reported_in_one_run() {
        let (result, diags, _) = parse(r#"{"value": "big", "unit": "em"}"#, "");
        assert!(result.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidTokenValue; 2]);
    }

    #[test]
    fn unknown_members_are_reported_but_the_value_is_still_read() {
        let (result, diags, _) = parse(r#"{"value": 1, "unit": "px", "extra": 2}"#, "");
        assert!(result.is_some());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidPropertyShape]);
        assert!(diags.diagnostics()[0].message.contains("`extra`"));
    }

    #[test]
    fn curly_aliases_inside_a_value_are_rejected_with_advice() {
        let (result, diags, _) = parse(
            r#"{"value": "{base}", "unit": "{u}"}"#,
            r#", "base": {"$type": "number", "$value": 1}, "u": {"$type": "number", "$value": 1}"#,
        );
        assert!(result.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidTokenValue; 2]);
        assert!(
            diags.diagnostics()[0]
                .help
                .as_deref()
                .unwrap()
                .contains("$ref")
        );
    }

    #[test]
    fn a_reference_object_with_other_members_is_rejected() {
        let (result, diags, _) = parse(
            r##"{"value": {"$ref": "#/base/$value", "x": 1}, "unit": "px"}"##,
            r##", "base": {"$type": "number", "$value": 1}"##,
        );
        assert!(result.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidPropertyShape]);
    }

    #[test]
    fn an_unresolved_ref_is_left_to_the_reference_pass() {
        let (result, diags, _) = parse(r##"{"value": {"$ref": "#/nope"}, "unit": "px"}"##, "");
        assert!(result.is_none());
        // It was already reported as `UnresolvedReference` by `analyze`, so there is nothing more to say here.
        assert!(diags.diagnostics().is_empty());
    }
}
