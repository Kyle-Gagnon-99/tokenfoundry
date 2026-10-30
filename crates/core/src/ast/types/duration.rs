//! The `duration` module provides the AST representation and parsing logic for duration values in DTCG.

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
pub enum DurationUnit {
    S,
    Ms,
}

impl DurationUnit {
    const NAMES: &'static [&'static str] = &["s", "ms"];

    pub(crate) fn parse(s: &str) -> Option<Self> {
        match s {
            "s" => Some(DurationUnit::S),
            "ms" => Some(DurationUnit::Ms),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Duration {
    pub value: Node<ValueOrRef<f64>>,
    pub unit: Node<ValueOrRef<DurationUnit>>,
}

impl FromProp for Duration {
    fn from_prop(prop: PropId, a: &Analyzed, diags: &mut impl DiagnosticSink) -> Option<Self> {
        let Some(members) = a.members(prop) else {
            diags.emit(
                Diagnostic::new(DiagnosticCode::InvalidPropertyShape)
                    .with_message("a duration must be an object")
                    .primary(a.span(prop), "expected {\"value\": 16, \"unit\": \"s\"}"),
            );
            return None;
        };

        // The schema forbids other members. This is reported but does not stop the known ones from being read.
        for (key, &member) in members {
            if key != "value" && key != "unit" {
                diags.emit(
                    Diagnostic::new(DiagnosticCode::InvalidPropertyShape)
                        .with_message(format!("unknown duration property `{key}`"))
                        .primary(a.span(member), "a duration only has `value` and `unit`"),
                );
            }
        }

        let value = match a.member(prop, "value") {
            Some(slot) => number_slot(a, slot, "a duration `value`", diags),
            None => {
                report_missing(a, prop, "value", diags);
                None
            }
        };

        let unit = match a.member(prop, "unit") {
            Some(slot) => choice_slot(
                a,
                slot,
                "a duration `unit`",
                &DurationUnit::NAMES,
                DurationUnit::parse,
                diags,
            ),
            None => {
                report_missing(a, prop, "unit", diags);
                None
            }
        };

        Some(Duration {
            value: value?,
            unit: unit?,
        })
    }
}

fn report_missing(a: &Analyzed, prop: PropId, key: &str, diags: &mut impl DiagnosticSink) {
    let mut diag = Diagnostic::new(DiagnosticCode::InvalidPropertyShape)
        .with_message(format!("a duration is missing `{key}`"))
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

    /// Analyzes `{"t": {"$type": "duration", "$value": <value>}}` plus `extra` top-level members, and parses the
    /// `$value`.
    fn parse(value: &str, extra: &str) -> (Option<Duration>, DiagnosticStore, Analyzed) {
        let doc = format!(r#"{{"t": {{"$type": "duration", "$value": {value}}}{extra}}}"#);
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
        let result = Duration::from_prop(v, &analyzed, &mut diags);
        (result, diags, analyzed)
    }

    fn codes(d: &DiagnosticStore) -> Vec<DiagnosticCode> {
        d.diagnostics().iter().map(|d| d.code).collect()
    }

    fn ok(value: &str) -> Duration {
        let (result, diags, _) = parse(value, "");
        assert!(diags.diagnostics().is_empty(), "{:?}", diags.diagnostics());
        result.expect("a duration")
    }

    #[test]
    fn parses_seconds_and_milliseconds() {
        let s = ok(r#"{"value": 0.5, "unit": "s"}"#);
        assert_eq!(s.value.value, ValueOrRef::Value(0.5));
        assert_eq!(s.unit.value, ValueOrRef::Value(DurationUnit::S));

        let ms = ok(r#"{"value": 200, "unit": "ms"}"#);
        assert_eq!(ms.value.value, ValueOrRef::Value(200.0));
        assert_eq!(ms.unit.value, ValueOrRef::Value(DurationUnit::Ms));
    }

    #[test]
    fn zero_negative_and_exponent_values_are_numbers() {
        assert_eq!(
            ok(r#"{"value": 0, "unit": "ms"}"#).value.value,
            ValueOrRef::Value(0.0)
        );
        // The schema puts no lower bound on `value`.
        assert_eq!(
            ok(r#"{"value": -1, "unit": "ms"}"#).value.value,
            ValueOrRef::Value(-1.0)
        );
        assert_eq!(
            ok(r#"{"value": 1.5e2, "unit": "ms"}"#).value.value,
            ValueOrRef::Value(150.0)
        );
    }

    #[test]
    fn member_order_does_not_matter() {
        assert_eq!(
            ok(r#"{"unit": "s", "value": 1}"#).unit.value,
            ValueOrRef::Value(DurationUnit::S)
        );
    }

    #[test]
    fn nodes_keep_their_own_spans() {
        let (result, _, a) = parse(r#"{"value": 200, "unit": "ms"}"#, "");
        let d = result.unwrap();
        assert_ne!(d.value.span, d.unit.span);
        assert_eq!(d.value.span, a.span(d.value.id));
        assert_eq!(a.number(d.value.id), Some(200.0));
        assert_eq!(a.string(d.unit.id), Some("ms"));
    }

    #[test]
    fn a_ref_object_becomes_a_reference_to_the_final_literal() {
        let (result, diags, a) = parse(
            r##"{"value": {"$ref": "#/base/$value"}, "unit": {"$ref": "#/d/$value/unit"}}"##,
            r##", "base": {"$type": "number", "$value": 300}, "d": {"$type": "duration", "$value": {"value": 1, "unit": "s"}}"##,
        );
        assert!(diags.diagnostics().is_empty(), "{:?}", diags.diagnostics());
        let d = result.unwrap();

        let base = a
            .lowered
            .resolve_pointer(&JsonPointer::parse_pointer("/base/$value").unwrap())
            .unwrap();
        assert_eq!(d.value.value, ValueOrRef::Reference(base));
        assert_ne!(d.value.id, base);
        assert!(matches!(d.unit.value, ValueOrRef::Reference(_)));
    }

    #[test]
    fn a_ref_to_the_wrong_kind_of_literal_is_rejected_at_both_ends() {
        let (result, diags, _) = parse(
            r##"{"value": {"$ref": "#/s/$value"}, "unit": "ms"}"##,
            r##", "s": {"$type": "number", "$value": "fast"}"##,
        );
        assert!(result.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidTokenValue]);
        assert_eq!(diags.diagnostics()[0].labels.len(), 2);
    }

    #[test]
    fn a_ref_to_an_unknown_unit_is_rejected() {
        let (result, diags, _) = parse(
            r##"{"value": 1, "unit": {"$ref": "#/u/$value"}}"##,
            r##", "u": {"$type": "number", "$value": "min"}"##,
        );
        assert!(result.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidTokenValue]);
    }

    #[test]
    fn a_ref_to_a_token_instead_of_its_value_is_an_error_with_a_fix() {
        let (result, diags, _) = parse(
            r##"{"value": {"$ref": "#/base"}, "unit": "s"}"##,
            r##", "base": {"$type": "number", "$value": 1}"##,
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
    fn a_value_that_is_not_an_object_is_rejected() {
        for value in ["200", r#""200ms""#, "[200, \"ms\"]", "true"] {
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
    fn missing_members_are_each_reported() {
        let (result, diags, _) = parse("{}", "");
        assert!(result.is_none());
        assert_eq!(
            codes(&diags),
            [
                DiagnosticCode::InvalidPropertyShape,
                DiagnosticCode::InvalidPropertyShape
            ]
        );

        let (_, diags, _) = parse(r#"{"value": 1}"#, "");
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidPropertyShape]);
        assert!(diags.diagnostics()[0].help.is_some());
    }

    #[test]
    fn a_bad_unit_lists_the_choices() {
        let (result, diags, _) = parse(r#"{"value": 1, "unit": "min"}"#, "");
        assert!(result.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidTokenValue]);
        let message = &diags.diagnostics()[0].message;
        assert!(message.contains("\"s\"") && message.contains("\"ms\""));
    }

    #[test]
    fn a_bad_value_is_rejected() {
        for value in [r#""1""#, "null", "true", "[1]"] {
            let (result, diags, _) = parse(&format!(r#"{{"value": {value}, "unit": "s"}}"#), "");
            assert!(result.is_none(), "{value}");
            assert_eq!(
                codes(&diags),
                [DiagnosticCode::InvalidTokenValue],
                "{value}"
            );
        }
    }

    #[test]
    fn units_are_case_sensitive() {
        let (result, _, _) = parse(r#"{"value": 1, "unit": "MS"}"#, "");
        assert!(result.is_none());
    }

    #[test]
    fn every_problem_is_reported_in_one_run() {
        let (result, diags, _) = parse(r#"{"value": "x", "unit": "min"}"#, "");
        assert!(result.is_none());
        assert_eq!(codes(&diags).len(), 2);
    }

    #[test]
    fn unknown_members_are_reported_but_the_value_is_still_read() {
        let (result, diags, _) = parse(r#"{"value": 1, "unit": "s", "easing": "linear"}"#, "");
        assert!(result.is_some());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidPropertyShape]);
    }

    #[test]
    fn curly_aliases_inside_a_value_are_rejected_with_advice() {
        let (result, diags, _) = parse(r#"{"value": "{other}", "unit": "s"}"#, "");
        assert!(result.is_none());
        assert!(
            diags.diagnostics()[0]
                .help
                .as_ref()
                .unwrap()
                .contains("$ref")
        );
    }

    #[test]
    fn a_reference_object_with_other_members_is_rejected() {
        let (result, diags, _) = parse(
            r##"{"value": {"$ref": "#/x", "extra": 1}, "unit": "s"}"##,
            "",
        );
        assert!(result.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidPropertyShape]);
    }

    #[test]
    fn an_unresolved_ref_is_left_to_the_reference_pass() {
        let (result, diags, _) = parse(r##"{"value": {"$ref": "#/nope"}, "unit": "s"}"##, "");
        assert!(result.is_none());
        assert!(diags.diagnostics().is_empty());
    }
}
