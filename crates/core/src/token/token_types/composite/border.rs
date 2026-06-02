//! The `border` module defines the `BorderTokenValue` struct which represents the DTCG border token type.

use crate::{
    errors::DiagnosticCode,
    ir::{
        DiagnosticOwnership, InvalidReason, JsonObject, ParseState, RefAliasOrLiteral, TryFromJson,
    },
    token::token_types::{
        color::ColorTokenValue, composite::stroke_style::StrokeStyleTokenValue,
        dimension::DimensionTokenValue,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BorderColor(pub RefAliasOrLiteral<ColorTokenValue>);

impl<'a> TryFromJson<'a> for BorderColor {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> crate::ir::ParseState<Self> {
        match RefAliasOrLiteral::<ColorTokenValue>::try_from_json(ctx, path, value) {
            ParseState::Parsed(color) => ParseState::Parsed(Self(color)),
            ParseState::Invalid(inv) => {
                if inv.ownership == DiagnosticOwnership::Silent {
                    ctx.push_to_errors(DiagnosticCode::InvalidPropertyValue, format!("Expected a DTCG reference, JSON reference, or a color token but got {:?}", value), path.into());
                    return ParseState::invalid_emitted(InvalidReason::InvalidValue);
                }
                ParseState::Invalid(inv)
            }
            ParseState::NoMatch => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidPropertyValue,
                    format!(
                        "Expected a DTCG reference, JSON reference, or a color token but got {:?}",
                        value
                    ),
                    path.into(),
                );
                ParseState::invalid_emitted(InvalidReason::InvalidValue)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BorderWidth(pub RefAliasOrLiteral<DimensionTokenValue>);

impl<'a> TryFromJson<'a> for BorderWidth {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match RefAliasOrLiteral::<DimensionTokenValue>::try_from_json(ctx, path, value) {
            ParseState::Parsed(width) => ParseState::Parsed(Self(width)),
            ParseState::Invalid(inv) => {
                if inv.ownership == DiagnosticOwnership::Silent {
                    ctx.push_to_errors(DiagnosticCode::InvalidPropertyValue, format!("Expected a DTCG reference, JSON reference, or a dimension token but got {:?}", value), path.into());
                    return ParseState::invalid_emitted(InvalidReason::InvalidValue);
                }
                ParseState::Invalid(inv)
            }
            ParseState::NoMatch => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidPropertyValue,
                    format!(
                        "Expected a DTCG reference, JSON reference, or a dimension token but got {:?}",
                        value
                    ),
                    path.into(),
                );
                ParseState::invalid_emitted(InvalidReason::InvalidValue)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BorderStyle(pub RefAliasOrLiteral<StrokeStyleTokenValue>);

impl<'a> TryFromJson<'a> for BorderStyle {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match RefAliasOrLiteral::<StrokeStyleTokenValue>::try_from_json(ctx, path, value) {
            ParseState::Parsed(style) => ParseState::Parsed(Self(style)),
            ParseState::Invalid(inv) => {
                if inv.ownership == DiagnosticOwnership::Silent {
                    ctx.push_to_errors(DiagnosticCode::InvalidPropertyValue, format!("Expected a DTCG reference, JSON reference, or a stroke style token but got {:?}", value), path.into());
                    return ParseState::invalid_emitted(InvalidReason::InvalidValue);
                }
                ParseState::Invalid(inv)
            }
            ParseState::NoMatch => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidPropertyValue,
                    format!(
                        "Expected a DTCG reference, JSON reference, or a stroke style token but got {:?}",
                        value
                    ),
                    path.into(),
                );
                ParseState::invalid_emitted(InvalidReason::InvalidValue)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BorderTokenValue {
    pub color: BorderColor,
    pub width: BorderWidth,
    pub style: BorderStyle,
}

impl<'a> TryFromJson<'a> for BorderTokenValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        let obj = match value {
            serde_json::Value::Object(map) => JsonObject(map),
            _ => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidTokenValue,
                    format!("Expected an object for border token at {}", path),
                    path.into(),
                );
                return ParseState::invalid_emitted(InvalidReason::InvalidValue);
            }
        };

        let color = obj.required_field::<BorderColor>(ctx, path, "color");
        let width = obj.required_field::<BorderWidth>(ctx, path, "width");
        let style = obj.required_field::<BorderStyle>(ctx, path, "style");

        match (color, width, style) {
            (Some(color), Some(width), Some(style)) => ParseState::Parsed(Self {
                color,
                width,
                style,
            }),
            // Emitting `Silent` here because the individual field parsing will have already emitted errors
            _ => ParseState::invalid_silent(InvalidReason::InvalidValue),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{
        ParserContext,
        errors::DiagnosticCode,
        ir::{ParseState, RefAliasOrLiteral, TryFromJson},
    };

    fn test_ctx() -> ParserContext {
        ParserContext::new("{}".to_string())
    }

    #[test]
    fn border_color_parses_literal_ref_and_alias() {
        let mut ctx = test_ctx();

        let literal = BorderColor::try_from_json(
            &mut ctx,
            "#/token/color",
            &json!({
                "colorSpace": "srgb",
                "components": [1, 0, 0]
            }),
        );
        let by_ref = BorderColor::try_from_json(
            &mut ctx,
            "#/token/color",
            &json!({"$ref": "#/tokens/colors/brand"}),
        );
        let alias = BorderColor::try_from_json(&mut ctx, "#/token/color", &json!("{colors.brand}"));

        assert!(matches!(
            literal,
            ParseState::Parsed(BorderColor(RefAliasOrLiteral::Literal(_)))
        ));
        assert!(matches!(
            by_ref,
            ParseState::Parsed(BorderColor(RefAliasOrLiteral::Ref(_)))
        ));
        assert!(matches!(
            alias,
            ParseState::Parsed(BorderColor(RefAliasOrLiteral::Alias(_)))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn border_width_parses_literal_ref_and_alias() {
        let mut ctx = test_ctx();

        let literal = BorderWidth::try_from_json(
            &mut ctx,
            "#/token/width",
            &json!({"value": 1, "unit": "px"}),
        );
        let by_ref = BorderWidth::try_from_json(
            &mut ctx,
            "#/token/width",
            &json!({"$ref": "#/tokens/border-width/md"}),
        );
        let alias = BorderWidth::try_from_json(&mut ctx, "#/token/width", &json!("{size.sm}"));

        assert!(matches!(
            literal,
            ParseState::Parsed(BorderWidth(RefAliasOrLiteral::Literal(_)))
        ));
        assert!(matches!(
            by_ref,
            ParseState::Parsed(BorderWidth(RefAliasOrLiteral::Ref(_)))
        ));
        assert!(matches!(
            alias,
            ParseState::Parsed(BorderWidth(RefAliasOrLiteral::Alias(_)))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn border_style_parses_literal_ref_and_alias() {
        let mut ctx = test_ctx();

        let literal = BorderStyle::try_from_json(&mut ctx, "#/token/style", &json!("solid"));
        let by_ref = BorderStyle::try_from_json(
            &mut ctx,
            "#/token/style",
            &json!({"$ref": "#/tokens/stroke/style"}),
        );
        let alias = BorderStyle::try_from_json(&mut ctx, "#/token/style", &json!("{stroke.solid}"));

        assert!(matches!(
            literal,
            ParseState::Parsed(BorderStyle(RefAliasOrLiteral::Literal(_)))
        ));
        assert!(matches!(
            by_ref,
            ParseState::Parsed(BorderStyle(RefAliasOrLiteral::Ref(_)))
        ));
        assert!(matches!(
            alias,
            ParseState::Parsed(BorderStyle(RefAliasOrLiteral::Alias(_)))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn border_token_value_parses_full_literal_object() {
        let mut ctx = test_ctx();
        let input = json!({
            "color": {
                "colorSpace": "srgb",
                "components": [1, 0, 0]
            },
            "width": {"value": 1, "unit": "px"},
            "style": "dashed"
        });

        let state = BorderTokenValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::Parsed(_)));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn border_token_value_parses_refs_and_aliases() {
        let mut ctx = test_ctx();
        let input = json!({
            "color": {"$ref": "#/tokens/color/brand"},
            "width": "{size.border.md}",
            "style": {"$ref": "#/tokens/stroke/primary"}
        });

        let state = BorderTokenValue::try_from_json(&mut ctx, "#/token", &input);

        let parsed = match state {
            ParseState::Parsed(v) => v,
            _ => panic!("expected parsed border token value"),
        };

        assert!(matches!(parsed.color.0, RefAliasOrLiteral::Ref(_)));
        assert!(matches!(parsed.width.0, RefAliasOrLiteral::Alias(_)));
        assert!(matches!(parsed.style.0, RefAliasOrLiteral::Ref(_)));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn border_token_value_rejects_non_object() {
        let mut ctx = test_ctx();

        let state = BorderTokenValue::try_from_json(&mut ctx, "#/token", &json!("bad"));

        assert!(matches!(state, ParseState::Invalid(_)));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(ctx.errors[0].path, "#/token");
    }

    #[test]
    fn border_token_value_reports_missing_required_fields() {
        let mut ctx = test_ctx();
        let input = json!({
            "color": {
                "colorSpace": "srgb",
                "components": [1, 0, 0]
            },
            "width": {"value": 1, "unit": "px"}
        });

        let state = BorderTokenValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::Invalid(_)));
        assert!(ctx.errors.iter().any(|e| {
            e.code == DiagnosticCode::MissingRequiredProperty && e.path == "#/token/style"
        }));
    }

    #[test]
    fn border_token_value_reports_invalid_field_values() {
        let mut ctx = test_ctx();
        let input = json!({
            "color": true,
            "width": {"value": 1, "unit": "px"},
            "style": "solid"
        });

        let state = BorderTokenValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::Invalid(_)));
        assert!(ctx.errors.iter().any(|e| e.path == "#/token/color"));
    }
}
