//! The `stroke_style` module provides the AST representation of strokeStyle types in DTCG.
//!
//! A stroke style is one of the eight named styles, which mean what the CSS line styles mean, or an object with a
//! `dashArray` and a `lineCap`. The dash array is a non-empty list of dimensions, each a literal, a `$ref` or a
//! `{token}` alias, or one `$ref` to such a list. The line cap is `round`, `butt` or `square`, or a `$ref`.

use crate::{
    ast::{
        node::{Node, PropId},
        types::{FromProp, TokenType, dimension::Dimension},
        value::{
            SlotKind, ValueOrRef, ValueRefOrAlias, alias_help, checked_reference, choice_slot,
            classify_slot, composite_slot, missing_member, node, object_members, reject_unknown,
        },
    },
    diagnostics::{
        code::DiagnosticCode,
        diagnostic::{Diagnostic, DiagnosticSink, DiagnosticStore},
    },
    ir::pipeline::Analyzed,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum StrokeKeyword {
    Solid,
    Dashed,
    Dotted,
    Double,
    Groove,
    Ridge,
    Outset,
    Inset,
}

impl StrokeKeyword {
    const NAMES: [&'static str; 8] = [
        "solid", "dashed", "dotted", "double", "groove", "ridge", "outset", "inset",
    ];

    fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "solid" => Self::Solid,
            "dashed" => Self::Dashed,
            "dotted" => Self::Dotted,
            "double" => Self::Double,
            "groove" => Self::Groove,
            "ridge" => Self::Ridge,
            "outset" => Self::Outset,
            "inset" => Self::Inset,
            _ => return None,
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "lowercase")]
pub enum LineCap {
    Round,
    Butt,
    Square,
}

impl LineCap {
    const NAMES: [&'static str; 3] = ["round", "butt", "square"];

    pub(crate) fn parse(s: &str) -> Option<Self> {
        match s {
            "round" => Some(Self::Round),
            "butt" => Some(Self::Butt),
            "square" => Some(Self::Square),
            _ => None,
        }
    }
}

/// The dash lengths of a [`DashedStroke`], in order.
pub type DashArray = Vec<Node<ValueRefOrAlias<Dimension>>>;

#[derive(Debug, Clone, PartialEq)]
pub struct DashedStroke {
    /// A `Reference` holds the prop of the list it ends at.
    pub dash_array: Node<ValueOrRef<DashArray>>,
    pub line_cap: Node<ValueOrRef<LineCap>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StrokeStyle {
    Keyword(StrokeKeyword),
    Dashed(DashedStroke),
}

impl FromProp for StrokeStyle {
    fn from_prop(prop: PropId, a: &Analyzed, diags: &mut impl DiagnosticSink) -> Option<Self> {
        if a.members(prop).is_some() {
            return DashedStroke::from_prop(prop, a, diags).map(StrokeStyle::Dashed);
        }

        if let Some(keyword) = a.string(prop).and_then(StrokeKeyword::parse) {
            return Some(StrokeStyle::Keyword(keyword));
        }

        let names = StrokeKeyword::NAMES.map(|n| format!("\"{n}\"")).join(", ");
        diags.emit(
            Diagnostic::new(DiagnosticCode::InvalidTokenValue)
                .with_message("a stroke style must be a style name or an object")
                .primary(
                    a.span(prop),
                    "expected a style name or {\"dashArray\": [...], \"lineCap\": ...}",
                )
                .help(format!(
                    "the style names are {names}. {}",
                    alias_help(a, prop)
                )),
        );
        None
    }
}

impl DashedStroke {
    const KEYS: [&'static str; 2] = ["dashArray", "lineCap"];
}

impl FromProp for DashedStroke {
    fn from_prop(prop: PropId, a: &Analyzed, diags: &mut impl DiagnosticSink) -> Option<Self> {
        let members = object_members(
            a,
            prop,
            "stroke style",
            "{\"dashArray\": [...], \"lineCap\": \"round\"}",
            diags,
        )?;
        reject_unknown(a, members, "stroke style", &Self::KEYS, diags);

        let dash_array = match a.member(prop, "dashArray") {
            Some(slot) => dash_array_slot(a, slot, diags),
            None => {
                diags.emit(
                    missing_member(a, prop, "stroke style", "dashArray")
                        .help("`dashArray` and `lineCap` are both required"),
                );
                None
            }
        };
        let line_cap = match a.member(prop, "lineCap") {
            Some(slot) => choice_slot(
                a,
                slot,
                "a stroke style `lineCap`",
                &LineCap::NAMES,
                LineCap::parse,
                diags,
            ),
            None => {
                diags.emit(
                    missing_member(a, prop, "stroke style", "lineCap")
                        .help("`dashArray` and `lineCap` are both required"),
                );
                None
            }
        };

        Some(DashedStroke {
            dash_array: dash_array?,
            line_cap: line_cap?,
        })
    }
}

fn dash_array_slot(
    a: &Analyzed,
    slot: PropId,
    diags: &mut impl DiagnosticSink,
) -> Option<Node<ValueOrRef<DashArray>>> {
    match classify_slot(a, slot, diags) {
        SlotKind::Reference(target) => {
            let end = checked_reference(
                a,
                slot,
                target,
                "a list of dimensions",
                |p| dash_array_items(a, p, &mut DiagnosticStore::new()).is_some(),
                diags,
            )?;
            Some(node(a, slot, ValueOrRef::Reference(end)))
        }
        SlotKind::Broken => None,
        SlotKind::Literal => {
            let items = dash_array_items(a, slot, diags)?;
            Some(node(a, slot, ValueOrRef::Value(items)))
        }
    }
}

/// Reads `prop` as a non-empty list of dimensions. The resolver uses it again to read a list a `$ref` ends at.
pub(crate) fn dash_array_items(
    a: &Analyzed,
    prop: PropId,
    diags: &mut impl DiagnosticSink,
) -> Option<DashArray> {
    let Some(items) = a.items(prop) else {
        diags.emit(
            Diagnostic::new(DiagnosticCode::InvalidTokenValue)
                .with_message("a `dashArray` must be a list of dimensions")
                .primary(
                    a.span(prop),
                    "expected [{\"value\": 4, \"unit\": \"px\"}, ...]",
                )
                .help(alias_help(a, prop)),
        );
        return None;
    };
    if items.is_empty() {
        diags.emit(
            Diagnostic::new(DiagnosticCode::InvalidTokenValue)
                .with_message("a `dashArray` must have at least one dimension")
                .primary(a.span(prop), "add the length of a dash"),
        );
        return None;
    }
    // Every item is read, so one run reports every problem in the list.
    let parsed: Vec<_> = items
        .iter()
        .map(|&item| composite_slot::<Dimension>(a, item, TokenType::Dimension, diags))
        .collect();
    parsed.into_iter().collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::types::testing::{codes, ok, parse};

    const DASH: &str = r#"{"dashArray": [{"value": 4, "unit": "px"}, {"value": 2, "unit": "px"}], "lineCap": "round"}"#;

    #[test]
    fn every_keyword_parses() {
        for name in StrokeKeyword::NAMES {
            let s = ok::<StrokeStyle>("strokeStyle", &format!("\"{name}\""), "");
            assert_eq!(s, StrokeStyle::Keyword(StrokeKeyword::parse(name).unwrap()));
        }
    }

    #[test]
    fn an_unknown_keyword_lists_the_known_ones() {
        let (s, diags, _) = parse::<StrokeStyle>("strokeStyle", r#""wavy""#, "");
        assert!(s.is_none());
        let help = diags.diagnostics()[0].help.as_ref().unwrap();
        assert!(help.contains("\"dotted\""), "{help}");
    }

    #[test]
    fn other_shapes_are_rejected() {
        for v in ["4", "true", "[1]", "null"] {
            let (s, diags, _) = parse::<StrokeStyle>("strokeStyle", v, "");
            assert!(s.is_none(), "{v}");
            assert_eq!(codes(&diags), [DiagnosticCode::InvalidTokenValue], "{v}");
        }
    }

    #[test]
    fn an_object_reads_the_dash_array_and_line_cap() {
        let StrokeStyle::Dashed(d) = ok::<StrokeStyle>("strokeStyle", DASH, "") else {
            panic!("not dashed")
        };
        let ValueOrRef::Value(items) = &d.dash_array.value else {
            panic!("not a literal")
        };
        assert_eq!(items.len(), 2);
        assert_eq!(d.line_cap.value, ValueOrRef::Value(LineCap::Round));
    }

    #[test]
    fn every_line_cap_parses() {
        for cap in LineCap::NAMES {
            let v =
                format!(r#"{{"dashArray": [{{"value": 1, "unit": "px"}}], "lineCap": "{cap}"}}"#);
            ok::<StrokeStyle>("strokeStyle", &v, "");
        }
    }

    #[test]
    fn a_bad_line_cap_is_rejected() {
        let v = r#"{"dashArray": [{"value": 1, "unit": "px"}], "lineCap": "pointy"}"#;
        let (s, diags, _) = parse::<StrokeStyle>("strokeStyle", v, "");
        assert!(s.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidTokenValue]);
    }

    #[test]
    fn missing_members_are_all_reported() {
        let (s, diags, _) = parse::<StrokeStyle>("strokeStyle", "{}", "");
        assert!(s.is_none());
        assert_eq!(
            codes(&diags),
            [
                DiagnosticCode::InvalidPropertyShape,
                DiagnosticCode::InvalidPropertyShape
            ]
        );
    }

    #[test]
    fn an_unknown_member_is_reported_but_known_ones_are_still_read() {
        let v = r#"{"dashArray": [{"value": 1, "unit": "px"}], "lineCap": "butt", "extra": 1}"#;
        let (s, diags, _) = parse::<StrokeStyle>("strokeStyle", v, "");
        assert!(s.is_some());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidPropertyShape]);
    }

    #[test]
    fn an_empty_or_non_list_dash_array_is_rejected() {
        for dash in ["[]", "4", r#""4px""#] {
            let v = format!(r#"{{"dashArray": {dash}, "lineCap": "round"}}"#);
            let (s, diags, _) = parse::<StrokeStyle>("strokeStyle", &v, "");
            assert!(s.is_none(), "{dash}");
            assert_eq!(codes(&diags), [DiagnosticCode::InvalidTokenValue], "{dash}");
        }
    }

    #[test]
    fn every_bad_dash_is_reported_in_one_run() {
        let v = r#"{"dashArray": [4, {"value": 1, "unit": "px"}, "x"], "lineCap": "round"}"#;
        let (s, diags, _) = parse::<StrokeStyle>("strokeStyle", v, "");
        assert!(s.is_none());
        assert_eq!(diags.diagnostics().len(), 2);
    }

    #[test]
    fn a_dash_can_be_a_dimension_alias_or_reference() {
        let v = r##"{"dashArray": ["{gap}", {"$ref": "#/gap/$value"}, {"value": 1, "unit": "px"}], "lineCap": "square"}"##;
        let StrokeStyle::Dashed(d) = ok::<StrokeStyle>(
            "strokeStyle",
            v,
            r#", "gap": {"$type": "dimension", "$value": {"value": 3, "unit": "px"}}"#,
        ) else {
            panic!()
        };
        let ValueOrRef::Value(items) = &d.dash_array.value else {
            panic!()
        };
        assert!(matches!(items[0].value, ValueRefOrAlias::Alias(_)));
        assert!(matches!(items[1].value, ValueRefOrAlias::Reference(_)));
        assert!(matches!(items[2].value, ValueRefOrAlias::Value(_)));
    }

    #[test]
    fn a_dash_alias_to_the_wrong_type_is_reported() {
        let v = r#"{"dashArray": ["{n}"], "lineCap": "round"}"#;
        let (s, diags, _) = parse::<StrokeStyle>(
            "strokeStyle",
            v,
            r#", "n": {"$type": "number", "$value": 3}"#,
        );
        assert!(s.is_none());
        assert_eq!(diags.diagnostics()[0].labels.len(), 2);
    }

    #[test]
    fn the_whole_dash_array_can_be_a_reference() {
        let v = r##"{"dashArray": {"$ref": "#/base/$value/dashArray"}, "lineCap": {"$ref": "#/base/$value/lineCap"}}"##;
        let StrokeStyle::Dashed(d) = ok::<StrokeStyle>(
            "strokeStyle",
            v,
            &format!(r#", "base": {{"$type": "strokeStyle", "$value": {DASH}}}"#),
        ) else {
            panic!()
        };
        assert!(matches!(d.dash_array.value, ValueOrRef::Reference(_)));
        assert!(matches!(d.line_cap.value, ValueOrRef::Reference(_)));
    }

    #[test]
    fn a_dash_array_reference_to_a_non_list_is_reported_at_both_ends() {
        let v = r##"{"dashArray": {"$ref": "#/n/$value"}, "lineCap": "round"}"##;
        let (s, diags, _) = parse::<StrokeStyle>(
            "strokeStyle",
            v,
            r#", "n": {"$type": "number", "$value": 3}"#,
        );
        assert!(s.is_none());
        assert_eq!(diags.diagnostics()[0].labels.len(), 2);
    }
}
