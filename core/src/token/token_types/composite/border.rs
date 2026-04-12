//! The `border` module defines the `BorderTokenValue` struct, which represents a border token value as defined in the DTCG specification.

use crate::{
    ir::{RefOr, TryFromJson, require_object},
    token::token_types::{
        color::ColorTokenValue,
        composite::{AliasOrLiteral, StrokeStyleTokenValue, parse_alias_or_literal},
        dimension::DimensionTokenValue,
    },
};

#[derive(Debug, Clone, PartialEq)]
pub struct BorderColor(pub AliasOrLiteral<ColorTokenValue>);

impl<'a> TryFromJson<'a> for BorderColor {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        parse_alias_or_literal(ctx, path, value).map(BorderColor)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BorderWidth(pub AliasOrLiteral<DimensionTokenValue>);

impl<'a> TryFromJson<'a> for BorderWidth {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        parse_alias_or_literal(ctx, path, value).map(BorderWidth)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BorderStyle(pub AliasOrLiteral<StrokeStyleTokenValue>);

impl<'a> TryFromJson<'a> for BorderStyle {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        parse_alias_or_literal(ctx, path, value).map(BorderStyle)
    }
}

/// The `BorderTokenValue` struct represents the value of a border token, which is a composite token that consists of a color, width, and style.
/// Each of these properties can be a reference to another token, an alias to another token, or a literal value.
#[derive(Debug, Clone, PartialEq)]
pub struct BorderTokenValue {
    /// The color of the border, which can be a reference to another token, an alias to another token, or a literal color value.
    pub color: RefOr<BorderColor>,
    /// The width of the border, which can be a reference to another token, an alias to another token, or a literal dimension value.
    pub width: RefOr<BorderWidth>,
    /// The style of the border, which can be a reference to another token, an alias to another token, or a literal stroke style value.
    pub style: RefOr<BorderStyle>,
}

impl<'a> TryFromJson<'a> for BorderTokenValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        let obj = require_object(ctx, path, value, "border token")?;
        let color = obj.required_field::<RefOr<BorderColor>>(ctx, path, "color");
        let width = obj.required_field::<RefOr<BorderWidth>>(ctx, path, "width");
        let style = obj.required_field::<RefOr<BorderStyle>>(ctx, path, "style");

        match (color, width, style) {
            (Some(color), Some(width), Some(style)) => Some(BorderTokenValue {
                color,
                width,
                style,
            }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        FileFormat, ParserContext,
        errors::DiagnosticCode,
        ir::{JsonPointer, JsonRef, RefOr, TokenAlias},
        token::token_types::composite::AliasOrLiteral,
    };
    use serde_json::json;

    fn parser_context() -> ParserContext {
        ParserContext::new("tests.json".into(), FileFormat::Json, String::new())
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum FieldMode {
        Literal,
        Alias,
        Ref,
    }

    fn color_json(mode: FieldMode) -> serde_json::Value {
        match mode {
            FieldMode::Literal => json!({
                "colorSpace": "srgb",
                "components": [0.1, 0.2, 0.3]
            }),
            FieldMode::Alias => json!("{palette.primary}"),
            FieldMode::Ref => json!({ "$ref": "#/border/color" }),
        }
    }

    fn width_json(mode: FieldMode) -> serde_json::Value {
        match mode {
            FieldMode::Literal => json!({ "value": 1, "unit": "px" }),
            FieldMode::Alias => json!("{border.width.sm}"),
            FieldMode::Ref => json!({ "$ref": "#/border/width" }),
        }
    }

    fn style_json(mode: FieldMode) -> serde_json::Value {
        match mode {
            FieldMode::Literal => json!("solid"),
            FieldMode::Alias => json!("{border.style.default}"),
            FieldMode::Ref => json!({ "$ref": "#/border/style" }),
        }
    }

    fn classify_color(value: &RefOr<BorderColor>) -> FieldMode {
        match value {
            RefOr::Ref(_) => FieldMode::Ref,
            RefOr::Literal(BorderColor(AliasOrLiteral::Alias(_))) => FieldMode::Alias,
            RefOr::Literal(BorderColor(AliasOrLiteral::Literal(_))) => FieldMode::Literal,
        }
    }

    fn classify_width(value: &RefOr<BorderWidth>) -> FieldMode {
        match value {
            RefOr::Ref(_) => FieldMode::Ref,
            RefOr::Literal(BorderWidth(AliasOrLiteral::Alias(_))) => FieldMode::Alias,
            RefOr::Literal(BorderWidth(AliasOrLiteral::Literal(_))) => FieldMode::Literal,
        }
    }

    fn classify_style(value: &RefOr<BorderStyle>) -> FieldMode {
        match value {
            RefOr::Ref(_) => FieldMode::Ref,
            RefOr::Literal(BorderStyle(AliasOrLiteral::Alias(_))) => FieldMode::Alias,
            RefOr::Literal(BorderStyle(AliasOrLiteral::Literal(_))) => FieldMode::Literal,
        }
    }

    #[test]
    fn parses_border_color_as_literal_and_alias() {
        let mut ctx = parser_context();

        let literal = BorderColor::try_from_json(
            &mut ctx,
            "/token/color",
            &json!({
                "colorSpace": "srgb",
                "components": [0.1, 0.2, 0.3]
            }),
        );
        let alias =
            BorderColor::try_from_json(&mut ctx, "/token/color", &json!("{palette.primary}"));

        assert!(matches!(
            literal,
            Some(BorderColor(AliasOrLiteral::Literal(_)))
        ));
        assert_eq!(
            alias,
            Some(BorderColor(AliasOrLiteral::Alias(
                TokenAlias::from_dtcg_alias("{palette.primary}").unwrap()
            )))
        );
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn parses_border_width_as_literal_and_alias() {
        let mut ctx = parser_context();

        let literal = BorderWidth::try_from_json(
            &mut ctx,
            "/token/width",
            &json!({ "value": 1, "unit": "px" }),
        );
        let alias =
            BorderWidth::try_from_json(&mut ctx, "/token/width", &json!("{border.width.sm}"));

        assert!(matches!(
            literal,
            Some(BorderWidth(AliasOrLiteral::Literal(_)))
        ));
        assert_eq!(
            alias,
            Some(BorderWidth(AliasOrLiteral::Alias(
                TokenAlias::from_dtcg_alias("{border.width.sm}").unwrap()
            )))
        );
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn parses_border_style_as_literal_and_alias() {
        let mut ctx = parser_context();

        let literal = BorderStyle::try_from_json(&mut ctx, "/token/style", &json!("solid"));
        let alias =
            BorderStyle::try_from_json(&mut ctx, "/token/style", &json!("{border.style.default}"));

        assert!(matches!(
            literal,
            Some(BorderStyle(AliasOrLiteral::Literal(_)))
        ));
        assert_eq!(
            alias,
            Some(BorderStyle(AliasOrLiteral::Alias(
                TokenAlias::from_dtcg_alias("{border.style.default}").unwrap()
            )))
        );
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn rejects_invalid_literal_border_subfields() {
        let mut ctx = parser_context();

        let color = BorderColor::try_from_json(&mut ctx, "/token/color", &json!(42));
        let width = BorderWidth::try_from_json(&mut ctx, "/token/width", &json!("wide"));
        let style = BorderStyle::try_from_json(&mut ctx, "/token/style", &json!(true));

        assert_eq!(color, None);
        assert_eq!(width, None);
        assert_eq!(style, None);
        assert_eq!(ctx.errors.len(), 3);
        assert_eq!(ctx.errors[0].path, "/token/color");
        assert_eq!(ctx.errors[1].path, "/token/width");
        assert_eq!(ctx.errors[2].path, "/token/style");
    }

    #[test]
    fn parses_border_token_with_literal_fields() {
        let mut ctx = parser_context();
        let value = json!({
            "color": {
                "colorSpace": "srgb",
                "components": [0.1, 0.2, 0.3]
            },
            "width": { "value": 1, "unit": "px" },
            "style": "solid"
        });

        let parsed = BorderTokenValue::try_from_json(&mut ctx, "/token", &value);

        match parsed {
            Some(BorderTokenValue {
                color,
                width,
                style,
            }) => {
                assert!(matches!(
                    color,
                    RefOr::Literal(BorderColor(AliasOrLiteral::Literal(_)))
                ));
                assert!(matches!(
                    width,
                    RefOr::Literal(BorderWidth(AliasOrLiteral::Literal(_)))
                ));
                assert!(matches!(
                    style,
                    RefOr::Literal(BorderStyle(AliasOrLiteral::Literal(_)))
                ));
            }
            None => panic!("expected border token to parse successfully"),
        }
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn parses_border_token_with_field_references() {
        let mut ctx = parser_context();
        let value = json!({
            "color": { "$ref": "#/border/color" },
            "width": { "$ref": "#/border/width" },
            "style": { "$ref": "#/border/style" }
        });

        let parsed = BorderTokenValue::try_from_json(&mut ctx, "/token", &value).unwrap();

        assert_eq!(
            parsed.color,
            RefOr::Ref(JsonRef::new_local_pointer(
                "#/border/color".into(),
                JsonPointer::from("#/border/color"),
            ))
        );
        assert_eq!(
            parsed.width,
            RefOr::Ref(JsonRef::new_local_pointer(
                "#/border/width".into(),
                JsonPointer::from("#/border/width"),
            ))
        );
        assert_eq!(
            parsed.style,
            RefOr::Ref(JsonRef::new_local_pointer(
                "#/border/style".into(),
                JsonPointer::from("#/border/style"),
            ))
        );
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn parses_all_border_field_mode_combinations() {
        let modes = [FieldMode::Literal, FieldMode::Alias, FieldMode::Ref];

        for color_mode in modes {
            for width_mode in modes {
                for style_mode in modes {
                    let mut ctx = parser_context();
                    let value = json!({
                        "color": color_json(color_mode),
                        "width": width_json(width_mode),
                        "style": style_json(style_mode),
                    });

                    let parsed = BorderTokenValue::try_from_json(&mut ctx, "/token", &value)
                        .unwrap_or_else(|| {
                            panic!(
                                "expected border token to parse for combination {:?}/{:?}/{:?}; errors: {:?}",
                                color_mode, width_mode, style_mode, ctx.errors
                            )
                        });

                    assert_eq!(classify_color(&parsed.color), color_mode);
                    assert_eq!(classify_width(&parsed.width), width_mode);
                    assert_eq!(classify_style(&parsed.style), style_mode);
                    assert!(
                        ctx.errors.is_empty(),
                        "unexpected errors for combination {:?}/{:?}/{:?}: {:?}",
                        color_mode,
                        width_mode,
                        style_mode,
                        ctx.errors
                    );
                }
            }
        }
    }

    #[test]
    fn reports_missing_required_border_fields() {
        let mut ctx = parser_context();

        let parsed = BorderTokenValue::try_from_json(&mut ctx, "/token", &json!({}));

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 3);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[0].path, "/token/color");
        assert_eq!(ctx.errors[1].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[1].path, "/token/width");
        assert_eq!(ctx.errors[2].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[2].path, "/token/style");
    }

    #[test]
    fn reports_invalid_border_field_and_missing_peers() {
        let mut ctx = parser_context();

        let parsed = BorderTokenValue::try_from_json(
            &mut ctx,
            "/token",
            &json!({
                "color": 42
            }),
        );

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 3);
        assert_eq!(ctx.errors[0].path, "/token/color");
        assert_eq!(ctx.errors[1].path, "/token/width");
        assert_eq!(ctx.errors[2].path, "/token/style");
    }

    #[test]
    fn reports_invalid_top_level_shape_for_non_object_border_token() {
        let mut ctx = parser_context();

        let parsed = BorderTokenValue::try_from_json(&mut ctx, "/token", &json!("solid"));

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyType);
        assert_eq!(ctx.errors[0].path, "/token");
    }
}
