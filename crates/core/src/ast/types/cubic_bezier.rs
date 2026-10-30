//! The `cubic_bezier` module provides the AST representation and parsing logic for cubic bezier values in DTCG.
//!
//! The value is an array of exactly four numbers, `[P1x, P1y, P2x, P2y]`. The x coordinates must be between 0 and 1,
//! and the y coordinates can be any number. Each item can be a `{"$ref": ...}` object, and so can the whole array.
//! A range is checked on the number a reference ends at.

use crate::{
    ast::{
        node::{Node, PropId},
        types::FromProp,
        value::{SlotKind, ValueOrRef, checked_reference, classify_slot, node, number_slot},
    },
    diagnostics::{
        code::DiagnosticCode,
        diagnostic::{Diagnostic, DiagnosticSink},
    },
    ir::pipeline::Analyzed,
};

/// P1x and P2x are limited to `[0, 1]`.
fn is_x(index: usize) -> bool {
    index % 2 == 0
}

fn in_range(index: usize, n: f64) -> bool {
    !is_x(index) || (0.0..=1.0).contains(&n)
}

/// Reports an x coordinate outside `[0, 1]`. `end` is the literal a reference ended at, if there was one.
fn report_range(
    a: &Analyzed,
    index: usize,
    n: f64,
    at: PropId,
    end: Option<PropId>,
    diags: &mut impl DiagnosticSink,
) {
    let name = if index == 0 { "P1x" } else { "P2x" };
    let mut diag = Diagnostic::new(DiagnosticCode::InvalidTokenValue)
        .with_message(format!(
            "the x coordinate {name} of a cubic bezier must be between 0 and 1, but this is {n}"
        ))
        .primary(a.span(at), "expected a number between 0 and 1");
    if let Some(end) = end {
        diag = diag.secondary(a.span(end), "the reference ends here");
    }
    diags.emit(diag);
}

#[derive(Debug, Clone, PartialEq)]
pub struct CubicBezier(pub Node<ValueOrRef<[Node<ValueOrRef<f64>>; 4]>>);

impl FromProp for CubicBezier {
    fn from_prop(prop: PropId, a: &Analyzed, diags: &mut impl DiagnosticSink) -> Option<Self> {
        let vals = match classify_slot(a, prop, diags) {
            SlotKind::Broken => None,
            SlotKind::Reference(target) => {
                let end = checked_reference(
                    a,
                    prop,
                    target,
                    "an array of 4 numbers, with the x coordinates between 0 and 1",
                    |p| {
                        a.items(p).is_some_and(|items| {
                            items.len() == 4
                                && items.iter().enumerate().all(|(i, &item)| {
                                    a.number(item).is_some_and(|n| in_range(i, n))
                                })
                        })
                    },
                    diags,
                )?;
                Some(node(a, prop, ValueOrRef::Reference(end)))
            }
            SlotKind::Literal => {
                let Some(items) = a.items(prop) else {
                    diags.emit(
                        Diagnostic::new(DiagnosticCode::InvalidTokenValue)
                            .with_message("a cubic bezier must be an array of 4 numbers")
                            .primary(a.span(prop), "expected [0.5, 0, 0.5, 1]"),
                    );
                    return None;
                };
                if items.len() != 4 {
                    diags.emit(
                        Diagnostic::new(DiagnosticCode::InvalidTokenValue)
                            .with_message(format!(
                                "a cubic bezier must have exactly 4 entries, found {}",
                                items.len()
                            ))
                            .primary(a.span(prop), "expected 4 entries"),
                    );
                    return None;
                }

                // Every item is read, so one run reports every problem in the array.
                let parsed: Vec<_> = items
                    .iter()
                    .enumerate()
                    .map(|(i, &item)| coordinate(a, item, i, diags))
                    .collect();
                let [Some(x1), Some(y1), Some(x2), Some(y2)] = <[_; 4]>::try_from(parsed).ok()?
                else {
                    return None;
                };
                Some(node(a, prop, ValueOrRef::Value([x1, y1, x2, y2])))
            }
        };
        Some(CubicBezier(vals?))
    }
}

fn coordinate(
    a: &Analyzed,
    slot: PropId,
    index: usize,
    diags: &mut impl DiagnosticSink,
) -> Option<Node<ValueOrRef<f64>>> {
    let coordinate = number_slot(a, slot, "a cubic bezier coordinate", diags)?;
    let (n, end) = match coordinate.value {
        ValueOrRef::Value(n) => (n, None),
        ValueOrRef::Reference(end) => (a.number(end)?, Some(end)),
    };
    if in_range(index, n) {
        return Some(coordinate);
    }
    report_range(a, index, n, slot, end, diags);
    None
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

    /// Analyzes `{"t": {"$type": "cubicBezier", "$value": <value>}}` plus `extra` top-level members, and parses
    /// the `$value`.
    fn parse(value: &str, extra: &str) -> (Option<CubicBezier>, DiagnosticStore, Analyzed) {
        let doc = format!(r#"{{"t": {{"$type": "cubicBezier", "$value": {value}}}{extra}}}"#);
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
        let result = CubicBezier::from_prop(v, &analyzed, &mut diags);
        (result, diags, analyzed)
    }

    fn codes(d: &DiagnosticStore) -> Vec<DiagnosticCode> {
        d.diagnostics().iter().map(|d| d.code).collect()
    }

    fn numbers(c: &CubicBezier) -> Vec<f64> {
        let ValueOrRef::Value(items) = &c.0.value else {
            panic!("not a literal array")
        };
        items
            .iter()
            .map(|n| match n.value {
                ValueOrRef::Value(v) => v,
                ValueOrRef::Reference(_) => panic!("item is a reference"),
            })
            .collect()
    }

    fn ok(value: &str) -> CubicBezier {
        let (result, diags, _) = parse(value, "");
        assert!(diags.diagnostics().is_empty(), "{:?}", diags.diagnostics());
        result.expect("a cubic bezier")
    }

    fn rejected(value: &str) -> Vec<DiagnosticCode> {
        let (result, diags, _) = parse(value, "");
        assert!(result.is_none(), "{value} was accepted");
        codes(&diags)
    }

    #[test]
    fn parses_four_numbers() {
        assert_eq!(numbers(&ok("[0.5, 0, 0.5, 1]")), [0.5, 0.0, 0.5, 1.0]);
    }

    #[test]
    fn x_coordinates_include_both_bounds() {
        assert_eq!(numbers(&ok("[0, 0, 1, 1]")), [0.0, 0.0, 1.0, 1.0]);
    }

    #[test]
    fn y_coordinates_are_unbounded() {
        // Overshoot and bounce curves need y outside [0, 1].
        assert_eq!(numbers(&ok("[0.3, -2, 0.7, 5.5]")), [0.3, -2.0, 0.7, 5.5]);
        ok("[0, -1e300, 1, 1e300]");
    }

    #[test]
    fn x_coordinates_outside_zero_to_one_are_rejected() {
        for value in [
            "[1.1, 0, 0.5, 1]",
            "[-0.1, 0, 0.5, 1]",
            "[0.5, 0, 2, 1]",
            "[0.5, 0, -1, 1]",
        ] {
            assert_eq!(
                rejected(value),
                [DiagnosticCode::InvalidTokenValue],
                "{value}"
            );
        }
    }

    #[test]
    fn both_bad_x_coordinates_are_reported_in_one_run() {
        assert_eq!(rejected("[2, 0, 3, 1]").len(), 2);
    }

    #[test]
    fn the_wrong_number_of_entries_is_rejected() {
        for value in ["[]", "[0.5]", "[0.5, 0, 0.5]", "[0.5, 0, 0.5, 1, 1]"] {
            assert_eq!(
                rejected(value),
                [DiagnosticCode::InvalidTokenValue],
                "{value}"
            );
        }
    }

    #[test]
    fn a_value_that_is_not_an_array_is_rejected() {
        for value in ["1", r#""ease-in""#, "true", r#"{"x1": 0}"#] {
            assert_eq!(
                rejected(value),
                [DiagnosticCode::InvalidTokenValue],
                "{value}"
            );
        }
    }

    #[test]
    fn entries_that_are_not_numbers_are_each_reported() {
        assert_eq!(
            rejected(r#"[0.5, "0", 0.5, 1]"#),
            [DiagnosticCode::InvalidTokenValue]
        );
        assert_eq!(
            rejected(r#"["a", true, null, [1]]"#),
            [DiagnosticCode::InvalidTokenValue; 4]
        );
    }

    #[test]
    fn nodes_keep_their_own_spans() {
        let (result, _, a) = parse("[0.5, 0, 0.5, 1]", "");
        let c = result.unwrap();
        let ValueOrRef::Value(items) = &c.0.value else {
            panic!()
        };
        assert_ne!(items[0].span, items[1].span);
        assert_eq!(items[0].span, a.span(items[0].id));
        assert_eq!(c.0.span, a.span(c.0.id));
        assert_eq!(a.number(items[2].id), Some(0.5));
    }

    #[test]
    fn an_item_can_be_a_reference_to_a_number() {
        let (result, diags, a) = parse(
            r##"[{"$ref": "#/x/$value"}, 0, 0.5, {"$ref": "#/y/$value"}]"##,
            r##", "x": {"$type": "number", "$value": 0.25}, "y": {"$type": "number", "$value": 9}"##,
        );
        assert!(diags.diagnostics().is_empty(), "{:?}", diags.diagnostics());
        let c = result.unwrap();
        let ValueOrRef::Value(items) = &c.0.value else {
            panic!()
        };
        let ValueOrRef::Reference(end) = items[0].value else {
            panic!()
        };
        assert_eq!(a.number(end), Some(0.25));
        assert_ne!(items[0].id, end);
        assert!(matches!(items[3].value, ValueOrRef::Reference(_)));
    }

    #[test]
    fn the_whole_array_can_be_a_reference() {
        let (result, diags, a) = parse(
            r##"{"$ref": "#/base/$value"}"##,
            r##", "base": {"$type": "cubicBezier", "$value": [0.1, 0.2, 0.3, 0.4]}"##,
        );
        assert!(diags.diagnostics().is_empty(), "{:?}", diags.diagnostics());
        let ValueOrRef::Reference(end) = result.unwrap().0.value else {
            panic!()
        };
        assert_eq!(a.items(end).unwrap().len(), 4);
    }

    #[test]
    fn a_reference_cannot_smuggle_in_an_out_of_range_x() {
        // One item, labelled at both ends.
        let (result, diags, _) = parse(
            r##"[{"$ref": "#/n/$value"}, 0, 0.5, 1]"##,
            r##", "n": {"$type": "number", "$value": 7}"##,
        );
        assert!(result.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidTokenValue]);
        assert_eq!(diags.diagnostics()[0].labels.len(), 2);

        // The same number is fine as a y coordinate.
        let (result, diags, _) = parse(
            r##"[0, {"$ref": "#/n/$value"}, 0.5, 1]"##,
            r##", "n": {"$type": "number", "$value": 7}"##,
        );
        assert!(diags.diagnostics().is_empty());
        assert!(result.is_some());
    }

    #[test]
    fn a_referenced_array_must_have_four_numbers() {
        for array in [
            "[0, 0, 1]",
            "[0, 0, 1, 1, 1]",
            r#"[0, "a", 1, 1]"#,
            "[5, 0, 0, 0]",
        ] {
            let (result, diags, _) = parse(
                r##"{"$ref": "#/arr/$value"}"##,
                &format!(r##", "arr": {{"$type": "cubicBezier", "$value": {array}}}"##),
            );
            assert!(result.is_none(), "{array}");
            // The target is a bad value too, but only the reference pass's own check is reported here.
            assert!(
                diags
                    .diagnostics()
                    .iter()
                    .any(|d| d.code == DiagnosticCode::InvalidTokenValue),
                "{array}"
            );
        }
    }

    #[test]
    fn a_ref_to_a_token_instead_of_its_value_is_an_error_with_a_fix() {
        let (result, diags, _) = parse(
            r##"{"$ref": "#/base"}"##,
            r##", "base": {"$type": "cubicBezier", "$value": [0, 0, 1, 1]}"##,
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
    fn a_ref_to_a_non_number_item_is_rejected_at_both_ends() {
        let (result, diags, _) = parse(
            r##"[{"$ref": "#/s/$value"}, 0, 0.5, 1]"##,
            r##", "s": {"$type": "number", "$value": "slow"}"##,
        );
        assert!(result.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidTokenValue]);
        assert_eq!(diags.diagnostics()[0].labels.len(), 2);
    }

    #[test]
    fn curly_aliases_inside_the_array_are_rejected_with_advice() {
        let (result, diags, _) = parse(r#"["{x}", 0, 0.5, 1]"#, "");
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
        assert_eq!(
            rejected(r##"{"$ref": "#/x", "extra": 1}"##),
            [DiagnosticCode::InvalidPropertyShape]
        );
    }

    #[test]
    fn an_unresolved_ref_is_left_to_the_reference_pass() {
        let (result, diags, _) = parse(r##"{"$ref": "#/nope"}"##, "");
        assert!(result.is_none());
        assert!(diags.diagnostics().is_empty());
    }
}
