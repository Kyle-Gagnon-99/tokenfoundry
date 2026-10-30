//! The `typography` module provides the AST representation of typography types in DTCG.
//!
//! A typography is an object with `fontFamily`, `fontSize`, `fontWeight`, `letterSpacing` and `lineHeight`, all
//! required. Each is a whole value of its own type, so each can be a literal, a `$ref` that leads to one, or a
//! `{token}` alias to a token of that type. `lineHeight` is a number that multiplies the font size.

use crate::{
    ast::{
        node::{Node, PropId},
        types::{
            FromProp, TokenType, dimension::Dimension, font_family::FontFamily,
            font_weight::FontWeight, number::Number,
        },
        value::{ValueRefOrAlias, composite_field, object_members, reject_unknown},
    },
    diagnostics::diagnostic::DiagnosticSink,
    ir::pipeline::Analyzed,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Typography {
    pub font_family: Node<ValueRefOrAlias<FontFamily>>,
    pub font_size: Node<ValueRefOrAlias<Dimension>>,
    pub font_weight: Node<ValueRefOrAlias<FontWeight>>,
    pub letter_spacing: Node<ValueRefOrAlias<Dimension>>,
    pub line_height: Node<ValueRefOrAlias<Number>>,
}

impl Typography {
    const KEYS: [&'static str; 5] = [
        "fontFamily",
        "fontSize",
        "fontWeight",
        "letterSpacing",
        "lineHeight",
    ];
}

impl FromProp for Typography {
    fn from_prop(prop: PropId, a: &Analyzed, diags: &mut impl DiagnosticSink) -> Option<Self> {
        let members = object_members(
            a,
            prop,
            "typography",
            "{\"fontFamily\": ..., \"fontSize\": ..., \"fontWeight\": ..., \"letterSpacing\": ..., \"lineHeight\": ...}",
            diags,
        )?;
        reject_unknown(a, members, "typography", &Self::KEYS, diags);

        let o = "typography";
        let font_family =
            composite_field::<FontFamily>(a, prop, o, "fontFamily", TokenType::FontFamily, diags);
        let font_size =
            composite_field::<Dimension>(a, prop, o, "fontSize", TokenType::Dimension, diags);
        let font_weight =
            composite_field::<FontWeight>(a, prop, o, "fontWeight", TokenType::FontWeight, diags);
        let letter_spacing =
            composite_field::<Dimension>(a, prop, o, "letterSpacing", TokenType::Dimension, diags);
        let line_height =
            composite_field::<Number>(a, prop, o, "lineHeight", TokenType::Number, diags);

        Some(Typography {
            font_family: font_family?,
            font_size: font_size?,
            font_weight: font_weight?,
            letter_spacing: letter_spacing?,
            line_height: line_height?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ast::types::testing::{codes, ok, parse},
        diagnostics::code::DiagnosticCode,
    };

    const PX: &str = r#"{"value": 16, "unit": "px"}"#;

    fn json(font_weight: &str) -> String {
        format!(
            r#"{{"fontFamily": ["Inter", "sans-serif"], "fontSize": {PX}, "fontWeight": {font_weight}, "letterSpacing": {{"value": 0, "unit": "px"}}, "lineHeight": 1.5}}"#
        )
    }

    #[test]
    fn parses_a_typography() {
        let t = ok::<Typography>("typography", &json("700"), "");
        assert!(matches!(t.font_family.value, ValueRefOrAlias::Value(_)));
        assert!(matches!(t.font_weight.value, ValueRefOrAlias::Value(FontWeight(w)) if w == 700.0));
        assert!(matches!(t.line_height.value, ValueRefOrAlias::Value(Number(n)) if n == 1.5));
    }

    #[test]
    fn a_font_weight_can_be_a_name() {
        let t = ok::<Typography>("typography", &json(r#""bold""#), "");
        assert!(matches!(t.font_weight.value, ValueRefOrAlias::Value(FontWeight(w)) if w == 700.0));
    }

    #[test]
    fn all_five_members_are_required_and_reported_together() {
        let (t, diags, _) = parse::<Typography>("typography", "{}", "");
        assert!(t.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidPropertyShape; 5]);
    }

    #[test]
    fn a_non_object_is_rejected() {
        let (t, diags, _) = parse::<Typography>("typography", "12", "");
        assert!(t.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidPropertyShape]);
    }

    #[test]
    fn an_unknown_member_is_reported() {
        let v = format!(
            r#"{}, "textTransform": "none"}}"#,
            json("400").trim_end_matches('}')
        );
        let (t, diags, _) = parse::<Typography>("typography", &v, "");
        assert!(t.is_some());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidPropertyShape]);
    }

    #[test]
    fn members_can_be_aliases_and_references() {
        let v = r##"{"fontFamily": "{family}", "fontSize": "{size}", "fontWeight": {"$ref": "#/weight/$value"}, "letterSpacing": {"$ref": "#/size/$value"}, "lineHeight": "{lh}"}"##;
        let t = ok::<Typography>(
            "typography",
            v,
            &format!(
                r#", "family": {{"$type": "fontFamily", "$value": "Inter"}}, "size": {{"$type": "dimension", "$value": {PX}}}, "weight": {{"$type": "fontWeight", "$value": "bold"}}, "lh": {{"$type": "number", "$value": 1.2}}"#
            ),
        );
        assert!(matches!(t.font_family.value, ValueRefOrAlias::Alias(_)));
        assert!(matches!(t.font_size.value, ValueRefOrAlias::Alias(_)));
        assert!(matches!(t.font_weight.value, ValueRefOrAlias::Reference(_)));
        assert!(matches!(
            t.letter_spacing.value,
            ValueRefOrAlias::Reference(_)
        ));
        assert!(matches!(t.line_height.value, ValueRefOrAlias::Alias(_)));
    }

    #[test]
    fn a_line_height_alias_to_a_dimension_is_reported_at_both_ends() {
        let v = json("400").replace("1.5", r#""{d}""#);
        let (t, diags, _) = parse::<Typography>(
            "typography",
            &v,
            &format!(r#", "d": {{"$type": "dimension", "$value": {PX}}}"#),
        );
        assert!(t.is_none());
        assert_eq!(diags.diagnostics()[0].labels.len(), 2);
    }

    #[test]
    fn a_bad_weight_and_a_bad_size_are_both_reported() {
        let v = json("5000").replace(PX, "16");
        let (t, diags, _) = parse::<Typography>("typography", &v, "");
        assert!(t.is_none());
        assert_eq!(diags.diagnostics().len(), 2);
    }
}
