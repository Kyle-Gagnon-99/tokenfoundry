//! The `gradient` module provides the AST representation of gradient types in DTCG.
//!
//! A gradient is a non-empty list of stops. Each item is a stop object, a `$ref` that leads to another gradient, or
//! a `{token}` alias to a gradient token, in which case that gradient's stops are spliced in. Unlike a shadow, a
//! gradient is never a lone object.
//!
//! A stop has a `color` (a whole color value) and a `position` from 0 to 1 (a number, a `$ref` that leads to one,
//! or a `{token}` alias to a number token). The schema limits the position to that range, so a literal or a `$ref`
//! outside it is an error. An alias can't be checked here because the number it names isn't known yet, so the
//! resolver clamps that case.

use crate::{
    ast::{
        node::{Node, PropId},
        types::{FromProp, TokenType, color::Color, number::Number},
        value::{
            SlotKind, ValueRefOrAlias, alias_target, checked_reference, classify_slot,
            composite_field, node, object_members, reject_unknown,
        },
    },
    diagnostics::{
        code::DiagnosticCode,
        diagnostic::{Diagnostic, DiagnosticSink, DiagnosticStore},
    },
    ir::{pipeline::Analyzed, refs::is_alias_like},
};

#[derive(Debug, Clone, PartialEq)]
pub struct GradientStop {
    pub color: Node<ValueRefOrAlias<Color>>,
    /// In `[0, 1]` unless it is an alias.
    pub position: Node<ValueRefOrAlias<Number>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Gradient {
    /// In the order written. A `Reference` or `Alias` entry leads to a whole gradient.
    pub stops: Vec<Node<ValueRefOrAlias<GradientStop>>>,
}

impl FromProp for Gradient {
    fn from_prop(prop: PropId, a: &Analyzed, diags: &mut impl DiagnosticSink) -> Option<Self> {
        let Some(items) = a.items(prop) else {
            diags.emit(
                Diagnostic::new(DiagnosticCode::InvalidTokenValue)
                    .with_message("a gradient must be a list of stops")
                    .primary(
                        a.span(prop),
                        "expected [{\"color\": ..., \"position\": 0}, ...]",
                    )
                    .help("a gradient with one stop is still a list of one"),
            );
            return None;
        };

        if items.is_empty() {
            diags.emit(
                Diagnostic::new(DiagnosticCode::InvalidTokenValue)
                    .with_message("a gradient must have at least one stop")
                    .primary(a.span(prop), "add a stop"),
            );
            return None;
        }

        // Every item is read, so one run reports every problem in the list.
        let stops: Vec<_> = items.iter().map(|&item| entry(a, item, diags)).collect();
        Some(Gradient {
            stops: stops.into_iter().collect::<Option<_>>()?,
        })
    }
}

/// One item of a gradient: a stop object, or a reference to a gradient token.
fn entry(
    a: &Analyzed,
    item: PropId,
    diags: &mut impl DiagnosticSink,
) -> Option<Node<ValueRefOrAlias<GradientStop>>> {
    match classify_slot(a, item, diags) {
        SlotKind::Broken => None,
        SlotKind::Reference(target) => {
            let end = checked_reference(
                a,
                item,
                target,
                "a valid `gradient` value",
                |p| Gradient::from_prop(p, a, &mut DiagnosticStore::new()).is_some(),
                diags,
            )?;
            Some(node(a, item, ValueRefOrAlias::Reference(end)))
        }
        SlotKind::Literal => match a.string(item) {
            Some(s) if is_alias_like(s) => {
                let target = alias_target(a, item, TokenType::Gradient, diags)?;
                Some(node(a, item, ValueRefOrAlias::Alias(target)))
            }
            _ => {
                let stop = GradientStop::from_prop(item, a, diags)?;
                Some(node(a, item, ValueRefOrAlias::Value(stop)))
            }
        },
    }
}

impl GradientStop {
    const KEYS: [&'static str; 2] = ["color", "position"];
}

impl FromProp for GradientStop {
    fn from_prop(prop: PropId, a: &Analyzed, diags: &mut impl DiagnosticSink) -> Option<Self> {
        let members = object_members(
            a,
            prop,
            "gradient stop",
            "{\"color\": ..., \"position\": 0.5}",
            diags,
        )?;
        reject_unknown(a, members, "gradient stop", &Self::KEYS, diags);

        let color =
            composite_field::<Color>(a, prop, "gradient stop", "color", TokenType::Color, diags);
        let position = composite_field::<Number>(
            a,
            prop,
            "gradient stop",
            "position",
            TokenType::Number,
            diags,
        )
        .filter(|p| position_in_range(a, p, diags));

        Some(GradientStop {
            color: color?,
            position: position?,
        })
    }
}

/// Checks a literal or referenced position. An alias is left for the resolver, see the module docs.
fn position_in_range(
    a: &Analyzed,
    position: &Node<ValueRefOrAlias<Number>>,
    diags: &mut impl DiagnosticSink,
) -> bool {
    let (n, end) = match &position.value {
        ValueRefOrAlias::Value(Number(n)) => (*n, None),
        ValueRefOrAlias::Reference(end) => match a.number(*end) {
            Some(n) => (n, Some(*end)),
            None => return true,
        },
        ValueRefOrAlias::Alias(_) => return true,
    };
    if (0.0..=1.0).contains(&n) {
        return true;
    }

    let mut d = Diagnostic::new(DiagnosticCode::InvalidTokenValue)
        .with_message(format!(
            "a gradient stop position must be between 0 and 1, not {n}"
        ))
        .primary(position.span.clone(), "out of range")
        .help("0 is the start of the gradient and 1 is the end");
    if let Some(end) = end {
        d = d.secondary(a.span(end), "the reference ends here");
    }
    diags.emit(d);
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::types::testing::{codes, ok, parse};

    const RED: &str = r#"{"colorSpace": "srgb", "components": [1, 0, 0]}"#;

    fn stop(position: &str) -> String {
        format!(r#"{{"color": {RED}, "position": {position}}}"#)
    }

    fn list(stops: &[String]) -> String {
        format!("[{}]", stops.join(", "))
    }

    #[test]
    fn parses_stops_in_order() {
        let g = ok::<Gradient>("gradient", &list(&[stop("0"), stop("0.5"), stop("1")]), "");
        assert_eq!(g.stops.len(), 3);
        let positions: Vec<_> = g
            .stops
            .iter()
            .map(|s| match &s.value {
                ValueRefOrAlias::Value(s) => match s.position.value {
                    ValueRefOrAlias::Value(Number(n)) => n,
                    _ => panic!(),
                },
                _ => panic!(),
            })
            .collect();
        assert_eq!(positions, [0.0, 0.5, 1.0]);
    }

    #[test]
    fn a_lone_stop_is_a_list_of_one() {
        assert_eq!(
            ok::<Gradient>("gradient", &list(&[stop("0")]), "")
                .stops
                .len(),
            1
        );
    }

    #[test]
    fn a_gradient_must_be_a_non_empty_list() {
        for v in ["[]", &stop("0"), "4", r#""red""#] {
            let (g, diags, _) = parse::<Gradient>("gradient", v, "");
            assert!(g.is_none(), "{v}");
            assert_eq!(codes(&diags), [DiagnosticCode::InvalidTokenValue], "{v}");
        }
    }

    #[test]
    fn a_position_outside_zero_to_one_is_rejected() {
        for p in ["-0.1", "1.5", "50"] {
            let (g, diags, _) = parse::<Gradient>("gradient", &list(&[stop(p)]), "");
            assert!(g.is_none(), "{p}");
            assert_eq!(codes(&diags), [DiagnosticCode::InvalidTokenValue], "{p}");
        }
    }

    #[test]
    fn a_stop_needs_both_members() {
        let (g, diags, _) = parse::<Gradient>("gradient", "[{}]", "");
        assert!(g.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidPropertyShape; 2]);
    }

    #[test]
    fn an_unknown_stop_member_is_reported() {
        let v = format!(r#"[{{"color": {RED}, "position": 0, "opacity": 1}}]"#);
        let (g, diags, _) = parse::<Gradient>("gradient", &v, "");
        assert!(g.is_some());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidPropertyShape]);
    }

    #[test]
    fn every_bad_stop_is_reported_in_one_run() {
        let (g, diags, _) =
            parse::<Gradient>("gradient", &list(&[stop("2"), stop("0"), stop("-1")]), "");
        assert!(g.is_none());
        assert_eq!(diags.diagnostics().len(), 2);
    }

    #[test]
    fn stop_members_can_be_aliases_and_references() {
        let v = format!(
            r##"[{{"color": "{{red}}", "position": {{"$ref": "#/half/$value"}}}}, {{"color": {{"$ref": "#/red/$value"}}, "position": "{{half}}"}}]"##
        );
        let g = ok::<Gradient>(
            "gradient",
            &v,
            &format!(
                r#", "red": {{"$type": "color", "$value": {RED}}}, "half": {{"$type": "number", "$value": 0.5}}"#
            ),
        );
        let ValueRefOrAlias::Value(first) = &g.stops[0].value else {
            panic!()
        };
        assert!(matches!(first.color.value, ValueRefOrAlias::Alias(_)));
        assert!(matches!(
            first.position.value,
            ValueRefOrAlias::Reference(_)
        ));
        let ValueRefOrAlias::Value(second) = &g.stops[1].value else {
            panic!()
        };
        assert!(matches!(second.color.value, ValueRefOrAlias::Reference(_)));
        assert!(matches!(second.position.value, ValueRefOrAlias::Alias(_)));
    }

    #[test]
    fn a_referenced_position_must_also_be_in_range() {
        let v = format!(r##"[{{"color": {RED}, "position": {{"$ref": "#/big/$value"}}}}]"##);
        let (g, diags, _) = parse::<Gradient>(
            "gradient",
            &v,
            r#", "big": {"$type": "number", "$value": 3}"#,
        );
        assert!(g.is_none());
        assert_eq!(diags.diagnostics()[0].labels.len(), 2);
    }

    #[test]
    fn a_position_alias_to_a_dimension_is_reported_at_both_ends() {
        let v = format!(r#"[{{"color": {RED}, "position": "{{d}}"}}]"#);
        let (g, diags, _) = parse::<Gradient>(
            "gradient",
            &v,
            r#", "d": {"$type": "dimension", "$value": {"value": 1, "unit": "px"}}"#,
        );
        assert!(g.is_none());
        assert_eq!(diags.diagnostics()[0].labels.len(), 2);
    }

    #[test]
    fn an_item_can_reference_or_alias_another_gradient() {
        let v = format!(
            r##"["{{base}}", {{"$ref": "#/base/$value"}}, {}]"##,
            stop("1")
        );
        let g = ok::<Gradient>(
            "gradient",
            &v,
            &format!(
                r#", "base": {{"$type": "gradient", "$value": {}}}"#,
                list(&[stop("0")])
            ),
        );
        assert!(matches!(g.stops[0].value, ValueRefOrAlias::Alias(_)));
        assert!(matches!(g.stops[1].value, ValueRefOrAlias::Reference(_)));
        assert!(matches!(g.stops[2].value, ValueRefOrAlias::Value(_)));
    }

    #[test]
    fn an_item_alias_to_a_shadow_is_reported() {
        let (g, diags, _) = parse::<Gradient>(
            "gradient",
            r#"["{s}"]"#,
            &format!(r#", "s": {{"$type": "color", "$value": {RED}}}"#),
        );
        assert!(g.is_none());
        assert_eq!(diags.diagnostics()[0].labels.len(), 2);
    }
}
