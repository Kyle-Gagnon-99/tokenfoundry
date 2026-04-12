//! The `gradient` module defines the data structures for gradient tokens defined in the DTCG specification.

use crate::{
    errors::DiagnosticCode,
    ir::{RefOr, TryFromJson, require_array, require_object},
    token::token_types::{
        color::ColorTokenValue, composite::AliasOrLiteral, number::NumberTokenValue,
    },
};

#[derive(Debug, Clone, PartialEq)]
pub struct GradientObjectColor(pub AliasOrLiteral<ColorTokenValue>);

impl<'a> TryFromJson<'a> for GradientObjectColor {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        AliasOrLiteral::<ColorTokenValue>::try_from_json(ctx, path, value).map(GradientObjectColor)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GradientObjectPosition(pub AliasOrLiteral<NumberTokenValue>);

impl<'a> TryFromJson<'a> for GradientObjectPosition {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        AliasOrLiteral::<NumberTokenValue>::try_from_json(ctx, path, value)
            .map(GradientObjectPosition)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GradientObject {
    pub color: RefOr<GradientObjectColor>,
    pub position: RefOr<GradientObjectPosition>,
}

impl<'a> TryFromJson<'a> for GradientObject {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        let obj = require_object(ctx, path, value, "gradient token")?;
        let color = obj.required_field::<RefOr<GradientObjectColor>>(ctx, path, "color");
        let position = obj.required_field::<RefOr<GradientObjectPosition>>(ctx, path, "position");

        match (color, position) {
            (Some(color), Some(position)) => Some(GradientObject { color, position }),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct GradientTokenValue(pub Vec<AliasOrLiteral<GradientObject>>);

impl<'a> TryFromJson<'a> for GradientTokenValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        let value = require_array(ctx, path, value)?;
        let mut items = Vec::new();
        for (index, item) in value.iter().enumerate() {
            let item_path = format!("{path}/{index}");
            let gradient_object =
                AliasOrLiteral::<GradientObject>::try_from_json(ctx, &item_path, item);
            if let Some(gradient_object) = gradient_object {
                items.push(gradient_object);
            } else {
                // If any item in the array is invalid, we skip the entire token and return None
                ctx.push_to_errors(
                    DiagnosticCode::InvalidTokenType,
                    format!("Invalid gradient object at {}", item_path),
                    path.into(),
                );
                return None;
            }
        }
        Some(GradientTokenValue(items))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        FileFormat, ParserContext,
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
                "components": [1.0, 0.0, 0.0]
            }),
            FieldMode::Alias => json!("{palette.red}"),
            FieldMode::Ref => json!({ "$ref": "#/gradient/color" }),
        }
    }

    fn position_json(mode: FieldMode) -> serde_json::Value {
        match mode {
            FieldMode::Literal => json!(25),
            FieldMode::Alias => json!("{gradient.position.start}"),
            FieldMode::Ref => json!({ "$ref": "#/gradient/position" }),
        }
    }

    fn classify_color(value: &RefOr<GradientObjectColor>) -> FieldMode {
        match value {
            RefOr::Ref(_) => FieldMode::Ref,
            RefOr::Literal(GradientObjectColor(AliasOrLiteral::Alias(_))) => FieldMode::Alias,
            RefOr::Literal(GradientObjectColor(AliasOrLiteral::Literal(_))) => FieldMode::Literal,
        }
    }

    fn classify_position(value: &RefOr<GradientObjectPosition>) -> FieldMode {
        match value {
            RefOr::Ref(_) => FieldMode::Ref,
            RefOr::Literal(GradientObjectPosition(AliasOrLiteral::Alias(_))) => FieldMode::Alias,
            RefOr::Literal(GradientObjectPosition(AliasOrLiteral::Literal(_))) => {
                FieldMode::Literal
            }
        }
    }

    #[test]
    fn parses_gradient_object_color_and_position_as_literal_and_alias() {
        let mut ctx = parser_context();

        let color_literal = GradientObjectColor::try_from_json(
            &mut ctx,
            "/token/color",
            &color_json(FieldMode::Literal),
        );
        let color_alias = GradientObjectColor::try_from_json(
            &mut ctx,
            "/token/color",
            &color_json(FieldMode::Alias),
        );
        let position_literal = GradientObjectPosition::try_from_json(
            &mut ctx,
            "/token/position",
            &position_json(FieldMode::Literal),
        );
        let position_alias = GradientObjectPosition::try_from_json(
            &mut ctx,
            "/token/position",
            &position_json(FieldMode::Alias),
        );

        assert!(matches!(
            color_literal,
            Some(GradientObjectColor(AliasOrLiteral::Literal(_)))
        ));
        assert_eq!(
            color_alias,
            Some(GradientObjectColor(AliasOrLiteral::Alias(
                TokenAlias::from_dtcg_alias("{palette.red}").unwrap()
            )))
        );
        assert!(matches!(
            position_literal,
            Some(GradientObjectPosition(AliasOrLiteral::Literal(_)))
        ));
        assert_eq!(
            position_alias,
            Some(GradientObjectPosition(AliasOrLiteral::Alias(
                TokenAlias::from_dtcg_alias("{gradient.position.start}").unwrap()
            )))
        );
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn rejects_invalid_gradient_object_color_and_position() {
        let mut ctx = parser_context();

        let color = GradientObjectColor::try_from_json(&mut ctx, "/token/color", &json!(42));
        let position =
            GradientObjectPosition::try_from_json(&mut ctx, "/token/position", &json!(true));

        assert_eq!(color, None);
        assert_eq!(position, None);
        assert_eq!(ctx.errors.len(), 2);
        assert_eq!(ctx.errors[0].path, "/token/color");
        assert_eq!(ctx.errors[1].path, "/token/position");
    }

    #[test]
    fn parses_all_gradient_object_field_mode_combinations() {
        let modes = [FieldMode::Literal, FieldMode::Alias, FieldMode::Ref];

        for color_mode in modes {
            for position_mode in modes {
                let mut ctx = parser_context();
                let value = json!({
                    "color": color_json(color_mode),
                    "position": position_json(position_mode),
                });

                let parsed = GradientObject::try_from_json(&mut ctx, "/token", &value)
                    .unwrap_or_else(|| {
                        panic!(
                            "expected gradient object to parse for combination {:?}/{:?}; errors: {:?}",
                            color_mode, position_mode, ctx.errors
                        )
                    });

                assert_eq!(classify_color(&parsed.color), color_mode);
                assert_eq!(classify_position(&parsed.position), position_mode);
                assert!(ctx.errors.is_empty());
            }
        }
    }

    #[test]
    fn reports_missing_required_gradient_object_fields() {
        let mut ctx = parser_context();

        let parsed = GradientObject::try_from_json(&mut ctx, "/token", &json!({}));

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 2);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[0].path, "/token/color");
        assert_eq!(ctx.errors[1].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[1].path, "/token/position");
    }

    #[test]
    fn reports_invalid_top_level_shape_for_gradient_object() {
        let mut ctx = parser_context();

        let parsed = GradientObject::try_from_json(&mut ctx, "/token", &json!("bad"));

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyType);
        assert_eq!(ctx.errors[0].path, "/token");
    }

    #[test]
    fn parses_gradient_token_value_with_literal_and_alias_items() {
        let mut ctx = parser_context();
        let value = json!([
            {
                "color": color_json(FieldMode::Literal),
                "position": position_json(FieldMode::Literal)
            },
            "{gradient.stop.primary}"
        ]);

        let parsed = GradientTokenValue::try_from_json(&mut ctx, "/token", &value);

        match parsed {
            Some(GradientTokenValue(items)) => {
                assert_eq!(items.len(), 2);
                assert!(matches!(items[0], AliasOrLiteral::Literal(_)));
                assert_eq!(
                    items[1],
                    AliasOrLiteral::Alias(
                        TokenAlias::from_dtcg_alias("{gradient.stop.primary}").unwrap()
                    )
                );
            }
            None => panic!("expected gradient token value to parse"),
        }
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn reports_invalid_gradient_array_item() {
        let mut ctx = parser_context();
        let value = json!([
            {
                "color": color_json(FieldMode::Literal),
                "position": position_json(FieldMode::Literal)
            },
            true
        ]);

        let parsed = GradientTokenValue::try_from_json(&mut ctx, "/token", &value);

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 2);
        assert_eq!(ctx.errors[0].path, "/token/1");
        assert_eq!(ctx.errors[1].code, DiagnosticCode::InvalidTokenType);
        assert_eq!(ctx.errors[1].path, "/token");
    }

    #[test]
    fn reports_invalid_top_level_shape_for_gradient_token_value() {
        let mut ctx = parser_context();

        let parsed = GradientTokenValue::try_from_json(&mut ctx, "/token", &json!({}));

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyType);
        assert_eq!(ctx.errors[0].path, "/token");
    }

    #[test]
    fn parses_gradient_object_with_direct_references() {
        let mut ctx = parser_context();
        let value = json!({
            "color": { "$ref": "#/gradient/color" },
            "position": { "$ref": "#/gradient/position" }
        });

        let parsed = GradientObject::try_from_json(&mut ctx, "/token", &value);

        assert_eq!(
            parsed,
            Some(GradientObject {
                color: RefOr::Ref(JsonRef::new_local_pointer(
                    "#/gradient/color".into(),
                    JsonPointer::from("#/gradient/color"),
                )),
                position: RefOr::Ref(JsonRef::new_local_pointer(
                    "#/gradient/position".into(),
                    JsonPointer::from("#/gradient/position"),
                )),
            })
        );
        assert!(ctx.errors.is_empty());
    }
}
