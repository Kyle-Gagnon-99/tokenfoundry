//! The `border` module provides the AST representation of border types in DTCG.
//!
//! A border is an object with `color`, `width` and `style`, all required. Each is a whole value of its own type
//! (color, dimension and strokeStyle), so each can be a literal, a `$ref` that leads to one, or a `{token}` alias
//! to a token of that type.

use crate::{
    ast::{
        node::{Node, PropId},
        types::{
            FromProp, TokenType, color::Color, dimension::Dimension, stroke_style::StrokeStyle,
        },
        value::{ValueRefOrAlias, composite_field, object_members, reject_unknown},
    },
    diagnostics::diagnostic::DiagnosticSink,
    ir::pipeline::Analyzed,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Border {
    pub color: Node<ValueRefOrAlias<Color>>,
    pub width: Node<ValueRefOrAlias<Dimension>>,
    pub style: Node<ValueRefOrAlias<StrokeStyle>>,
}

impl Border {
    const KEYS: [&'static str; 3] = ["color", "width", "style"];
}

impl FromProp for Border {
    fn from_prop(prop: PropId, a: &Analyzed, diags: &mut impl DiagnosticSink) -> Option<Self> {
        let members = object_members(
            a,
            prop,
            "border",
            "{\"color\": ..., \"width\": ..., \"style\": ...}",
            diags,
        )?;
        reject_unknown(a, members, "border", &Self::KEYS, diags);

        let color = composite_field::<Color>(a, prop, "border", "color", TokenType::Color, diags);
        let width =
            composite_field::<Dimension>(a, prop, "border", "width", TokenType::Dimension, diags);
        let style = composite_field::<StrokeStyle>(
            a,
            prop,
            "border",
            "style",
            TokenType::StrokeStyle,
            diags,
        );

        Some(Border {
            color: color?,
            width: width?,
            style: style?,
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

    const RED: &str = r#"{"colorSpace": "srgb", "components": [1, 0, 0]}"#;
    const PX: &str = r#"{"value": 1, "unit": "px"}"#;

    fn json(style: &str) -> String {
        format!(r#"{{"color": {RED}, "width": {PX}, "style": {style}}}"#)
    }

    #[test]
    fn parses_a_border_with_a_keyword_style() {
        let b = ok::<Border>("border", &json(r#""solid""#), "");
        assert!(matches!(
            b.style.value,
            ValueRefOrAlias::Value(StrokeStyle::Keyword(_))
        ));
    }

    #[test]
    fn parses_a_border_with_an_object_style() {
        let style = r#"{"dashArray": [{"value": 2, "unit": "px"}], "lineCap": "round"}"#;
        let b = ok::<Border>("border", &json(style), "");
        assert!(matches!(
            b.style.value,
            ValueRefOrAlias::Value(StrokeStyle::Dashed(_))
        ));
    }

    #[test]
    fn all_three_members_are_required_and_reported_together() {
        let (b, diags, _) = parse::<Border>("border", "{}", "");
        assert!(b.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidPropertyShape; 3]);
    }

    #[test]
    fn a_non_object_is_rejected() {
        let (b, diags, _) = parse::<Border>("border", r#""1px solid red""#, "");
        assert!(b.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidPropertyShape]);
    }

    #[test]
    fn an_unknown_member_is_reported() {
        let v = format!(r#"{{"color": {RED}, "width": {PX}, "style": "solid", "radius": 1}}"#);
        let (b, diags, _) = parse::<Border>("border", &v, "");
        assert!(b.is_some());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidPropertyShape]);
    }

    #[test]
    fn members_can_be_aliases_and_references() {
        let v = r##"{"color": "{red}", "width": {"$ref": "#/px/$value"}, "style": "{dashed}"}"##;
        let b = ok::<Border>(
            "border",
            v,
            &format!(
                r#", "red": {{"$type": "color", "$value": {RED}}}, "px": {{"$type": "dimension", "$value": {PX}}}, "dashed": {{"$type": "strokeStyle", "$value": "dashed"}}"#
            ),
        );
        assert!(matches!(b.color.value, ValueRefOrAlias::Alias(_)));
        assert!(matches!(b.width.value, ValueRefOrAlias::Reference(_)));
        assert!(matches!(b.style.value, ValueRefOrAlias::Alias(_)));
    }

    #[test]
    fn a_style_alias_to_a_color_token_is_reported_at_both_ends() {
        let v = format!(r#"{{"color": {RED}, "width": {PX}, "style": "{{c}}"}}"#);
        let (b, diags, _) = parse::<Border>(
            "border",
            &v,
            &format!(r#", "c": {{"$type": "color", "$value": {RED}}}"#),
        );
        assert!(b.is_none());
        assert_eq!(diags.diagnostics()[0].labels.len(), 2);
    }

    #[test]
    fn a_bad_style_is_reported_and_the_other_members_are_still_checked() {
        let v = format!(r#"{{"color": 1, "width": {PX}, "style": "wavy"}}"#);
        let (b, diags, _) = parse::<Border>("border", &v, "");
        assert!(b.is_none());
        assert_eq!(diags.diagnostics().len(), 2);
    }
}
