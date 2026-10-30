//! The `number` module provides the AST representation and parsing logic for numeric token values in DTCG.

use crate::{
    ast::{node::PropId, types::FromProp},
    diagnostics::{
        code::DiagnosticCode,
        diagnostic::{Diagnostic, DiagnosticSink},
    },
    ir::pipeline::Analyzed,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Number(pub f64);

impl FromProp for Number {
    fn from_prop(prop: PropId, a: &Analyzed, diags: &mut impl DiagnosticSink) -> Option<Self> {
        let Some(number) = a.number(prop) else {
            diags.emit(
                Diagnostic::new(DiagnosticCode::InvalidTokenValue)
                    .with_message("a number token's value must be a JSON number")
                    .primary(a.span(prop), "expected a number such as 16 or 0.5")
                    .help("numbers can't be quoted, so use 16 instead of \"16\""),
            );
            return None;
        };

        Some(Number(number))
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

    /// Analyzes `{"t": {"$type": "number", "$value": <value>}}` and parses the `$value`. Returns the result, the
    /// diagnostics from parsing only, the analysis, and the prop that was parsed.
    fn parse(value: &str) -> (Option<Number>, DiagnosticStore, Analyzed, PropId) {
        let doc = format!(r#"{{"t": {{"$type": "number", "$value": {value}}}}}"#);
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
        let result = Number::from_prop(v, &analyzed, &mut diags);
        (result, diags, analyzed, v)
    }

    fn ok(value: &str) -> f64 {
        let (result, diags, _, _) = parse(value);
        assert!(diags.diagnostics().is_empty(), "{:?}", diags.diagnostics());
        result.expect("a number").0
    }

    fn rejected(value: &str) -> Vec<DiagnosticCode> {
        let (result, diags, _, _) = parse(value);
        assert!(result.is_none(), "{value} was accepted");
        diags.diagnostics().iter().map(|d| d.code).collect()
    }

    #[test]
    fn parses_integers_and_decimals() {
        assert_eq!(ok("16"), 16.0);
        assert_eq!(ok("0.5"), 0.5);
        assert_eq!(ok("1000000"), 1_000_000.0);
    }

    #[test]
    fn zero_negative_and_exponent_values_are_numbers() {
        assert_eq!(ok("0"), 0.0);
        assert_eq!(ok("-4"), -4.0);
        assert_eq!(ok("-0.25"), -0.25);
        assert_eq!(ok("1.5e1"), 15.0);
        assert_eq!(ok("2E-2"), 0.02);
    }

    #[test]
    fn there_is_no_range_limit() {
        assert_eq!(ok("-1e300"), -1e300);
        assert_eq!(ok("1e300"), 1e300);
    }

    #[test]
    fn anything_that_is_not_a_json_number_is_rejected() {
        for value in [
            r#""16""#,
            r#""""#,
            "true",
            "false",
            "null",
            "[1]",
            "[]",
            r#"{"value": 1}"#,
            "{}",
        ] {
            assert_eq!(
                rejected(value),
                [DiagnosticCode::InvalidTokenValue],
                "{value}"
            );
        }
    }

    #[test]
    fn the_error_points_at_the_value() {
        let (result, diags, a, v) = parse(r#""sixteen""#);
        assert!(result.is_none());
        let diag = &diags.diagnostics()[0];
        assert_eq!(diag.labels.len(), 1);
        assert_eq!(diag.labels[0].span, a.span(v));
    }

    #[test]
    fn a_ref_object_is_not_a_literal_number() {
        // The token dispatcher handles `$ref` objects before a parser runs, so a parser that is handed one reports
        // it rather than guessing.
        assert_eq!(
            rejected(r##"{"$ref": "#/t2/$value"}"##),
            [DiagnosticCode::InvalidTokenValue]
        );
    }
}
