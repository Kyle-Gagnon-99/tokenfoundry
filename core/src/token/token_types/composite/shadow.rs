//! The `shadow` module defines the `ShadowTokenValue` struct, which represents a shadow token value as defined in the DTCG specification.

use crate::{
    errors::DiagnosticCode,
    ir::{RefOr, TryFromJson, require_object},
    token::token_types::{
        color::ColorTokenValue, composite::{AliasOrLiteral, RefAliasOrLiteral}, dimension::DimensionTokenValue,
    },
};

#[derive(Debug, Clone, PartialEq)]
pub struct ShadowColor(pub AliasOrLiteral<ColorTokenValue>);

impl<'a> TryFromJson<'a> for ShadowColor {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        AliasOrLiteral::try_from_json(ctx, path, value).map(ShadowColor)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShadowOffsetX(pub AliasOrLiteral<DimensionTokenValue>);

impl<'a> TryFromJson<'a> for ShadowOffsetX {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        AliasOrLiteral::try_from_json(ctx, path, value).map(ShadowOffsetX)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShadowOffsetY(pub AliasOrLiteral<DimensionTokenValue>);

impl<'a> TryFromJson<'a> for ShadowOffsetY {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        AliasOrLiteral::try_from_json(ctx, path, value).map(ShadowOffsetY)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShadowBlur(pub AliasOrLiteral<DimensionTokenValue>);

impl<'a> TryFromJson<'a> for ShadowBlur {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        AliasOrLiteral::try_from_json(ctx, path, value).map(ShadowBlur)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShadowSpread(pub AliasOrLiteral<DimensionTokenValue>);

impl<'a> TryFromJson<'a> for ShadowSpread {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        AliasOrLiteral::try_from_json(ctx, path, value).map(ShadowSpread)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShadowInset(pub bool);

impl<'a> TryFromJson<'a> for ShadowInset {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        bool::try_from_json(ctx, path, value).map(ShadowInset)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ShadowTokenSingleValue {
    pub color: RefOr<ShadowColor>,
    pub offset_x: RefOr<ShadowOffsetX>,
    pub offset_y: RefOr<ShadowOffsetY>,
    pub blur: RefOr<ShadowBlur>,
    pub spread: RefOr<ShadowSpread>,
    pub inset: Option<RefOr<ShadowInset>>,
}

impl<'a> TryFromJson<'a> for ShadowTokenSingleValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        let obj = require_object(ctx, path, value, "shadow token")?;
        let color = obj.required_field::<RefOr<ShadowColor>>(ctx, path, "color");
        let offset_x = obj.required_field::<RefOr<ShadowOffsetX>>(ctx, path, "offsetX");
        let offset_y = obj.required_field::<RefOr<ShadowOffsetY>>(ctx, path, "offsetY");
        let blur = obj.required_field::<RefOr<ShadowBlur>>(ctx, path, "blur");
        let spread = obj.required_field::<RefOr<ShadowSpread>>(ctx, path, "spread");
        let inset = obj.optional_field::<RefOr<ShadowInset>>(ctx, path, "inset");

        match (color, offset_x, offset_y, blur, spread) {
            (Some(color), Some(offset_x), Some(offset_y), Some(blur), Some(spread)) => {
                Some(ShadowTokenSingleValue {
                    color,
                    offset_x,
                    offset_y,
                    blur,
                    spread,
                    inset,
                })
            }
            _ => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidTokenValue, 
                    format!("Invalid shadow token value. Required fields are: color, offsetX, offsetY, blur, spread. Received value: {}", value), 
                    path.into()
                );
                None
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ShadowTokenValue {
    Single(RefAliasOrLiteral<ShadowTokenSingleValue>),
    Array(RefOr<Vec<RefAliasOrLiteral<ShadowTokenSingleValue>>>),
}

impl<'a> TryFromJson<'a> for ShadowTokenValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        match value {
            serde_json::Value::Object(_) => ShadowTokenSingleValue::try_from_json(ctx, path, value)
                .map(|single| ShadowTokenValue::Single(RefAliasOrLiteral::Literal(single))),
            serde_json::Value::Array(arr) => {
                let mut shadows = Vec::new();
                for (index, item) in arr.iter().enumerate() {
                    match RefAliasOrLiteral::<ShadowTokenSingleValue>::try_from_json(ctx, &format!("{}/{}", path, index), item) {
                        Some(single) => shadows.push(single),
                        None => {
                            // If any item in the array is invalid, we skip the entire token and return None
                            ctx.push_to_errors(
                                DiagnosticCode::InvalidTokenValue,
                                format!("Invalid shadow token value at index {}. Each item in the array must be a valid shadow token object. Received value: {}", index, item),
                                format!("{}/{}", path, index).into(),
                            );
                            return None;
                        }
                    }
                }
                Some(ShadowTokenValue::Array(RefOr::Literal(shadows)))
            }
            _ => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidTokenValue,
                    format!("Invalid shadow token value. Expected either an object or an array of objects, but found: {}", value),
                    path.into(),
                );
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        FileFormat, ParserContext,
        ir::{JsonPointer, JsonRef, RefOr, TokenAlias},
        token::token_types::composite::{AliasOrLiteral, RefAliasOrLiteral},
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
            FieldMode::Alias => json!("{shadow.color.default}"),
            FieldMode::Ref => json!({ "$ref": "#/shadow/color" }),
        }
    }

    fn dimension_json(mode: FieldMode, alias: &str, pointer: &str, value: i64) -> serde_json::Value {
        match mode {
            FieldMode::Literal => json!({ "value": value, "unit": "px" }),
            FieldMode::Alias => json!(alias),
            FieldMode::Ref => json!({ "$ref": pointer }),
        }
    }

    fn offset_x_json(mode: FieldMode) -> serde_json::Value {
        dimension_json(mode, "{shadow.offset.x}", "#/shadow/offsetX", 1)
    }

    fn offset_y_json(mode: FieldMode) -> serde_json::Value {
        dimension_json(mode, "{shadow.offset.y}", "#/shadow/offsetY", 2)
    }

    fn blur_json(mode: FieldMode) -> serde_json::Value {
        dimension_json(mode, "{shadow.blur.sm}", "#/shadow/blur", 4)
    }

    fn spread_json(mode: FieldMode) -> serde_json::Value {
        dimension_json(mode, "{shadow.spread.none}", "#/shadow/spread", 0)
    }

    fn classify_color(value: &RefOr<ShadowColor>) -> FieldMode {
        match value {
            RefOr::Ref(_) => FieldMode::Ref,
            RefOr::Literal(ShadowColor(AliasOrLiteral::Alias(_))) => FieldMode::Alias,
            RefOr::Literal(ShadowColor(AliasOrLiteral::Literal(_))) => FieldMode::Literal,
        }
    }

    fn classify_offset_x(value: &RefOr<ShadowOffsetX>) -> FieldMode {
        match value {
            RefOr::Ref(_) => FieldMode::Ref,
            RefOr::Literal(ShadowOffsetX(AliasOrLiteral::Alias(_))) => FieldMode::Alias,
            RefOr::Literal(ShadowOffsetX(AliasOrLiteral::Literal(_))) => FieldMode::Literal,
        }
    }

    fn classify_offset_y(value: &RefOr<ShadowOffsetY>) -> FieldMode {
        match value {
            RefOr::Ref(_) => FieldMode::Ref,
            RefOr::Literal(ShadowOffsetY(AliasOrLiteral::Alias(_))) => FieldMode::Alias,
            RefOr::Literal(ShadowOffsetY(AliasOrLiteral::Literal(_))) => FieldMode::Literal,
        }
    }

    fn classify_blur(value: &RefOr<ShadowBlur>) -> FieldMode {
        match value {
            RefOr::Ref(_) => FieldMode::Ref,
            RefOr::Literal(ShadowBlur(AliasOrLiteral::Alias(_))) => FieldMode::Alias,
            RefOr::Literal(ShadowBlur(AliasOrLiteral::Literal(_))) => FieldMode::Literal,
        }
    }

    fn classify_spread(value: &RefOr<ShadowSpread>) -> FieldMode {
        match value {
            RefOr::Ref(_) => FieldMode::Ref,
            RefOr::Literal(ShadowSpread(AliasOrLiteral::Alias(_))) => FieldMode::Alias,
            RefOr::Literal(ShadowSpread(AliasOrLiteral::Literal(_))) => FieldMode::Literal,
        }
    }

    #[test]
    fn parses_shadow_subfields_and_inset() {
        let mut ctx = parser_context();

        let color_literal = ShadowColor::try_from_json(&mut ctx, "/token/color", &color_json(FieldMode::Literal));
        let color_alias = ShadowColor::try_from_json(&mut ctx, "/token/color", &color_json(FieldMode::Alias));
        let offset_x_alias = ShadowOffsetX::try_from_json(&mut ctx, "/token/offsetX", &offset_x_json(FieldMode::Alias));
        let inset = ShadowInset::try_from_json(&mut ctx, "/token/inset", &json!(true));

        assert!(matches!(color_literal, Some(ShadowColor(AliasOrLiteral::Literal(_)))));
        assert_eq!(
            color_alias,
            Some(ShadowColor(AliasOrLiteral::Alias(
                TokenAlias::from_dtcg_alias("{shadow.color.default}").unwrap()
            )))
        );
        assert_eq!(
            offset_x_alias,
            Some(ShadowOffsetX(AliasOrLiteral::Alias(
                TokenAlias::from_dtcg_alias("{shadow.offset.x}").unwrap()
            )))
        );
        assert_eq!(inset, Some(ShadowInset(true)));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn rejects_invalid_shadow_subfields() {
        let mut ctx = parser_context();

        let color = ShadowColor::try_from_json(&mut ctx, "/token/color", &json!(42));
        let blur = ShadowBlur::try_from_json(&mut ctx, "/token/blur", &json!("wide"));
        let inset = ShadowInset::try_from_json(&mut ctx, "/token/inset", &json!("true"));

        assert_eq!(color, None);
        assert_eq!(blur, None);
        assert_eq!(inset, None);
        assert_eq!(ctx.errors.len(), 3);
        assert_eq!(ctx.errors[0].path, "/token/color");
        assert_eq!(ctx.errors[1].path, "/token/blur");
        assert_eq!(ctx.errors[2].path, "/token/inset");
    }

    #[test]
    fn parses_all_shadow_single_required_field_mode_combinations() {
        let modes = [FieldMode::Literal, FieldMode::Alias, FieldMode::Ref];

        for color_mode in modes {
            for offset_x_mode in modes {
                for offset_y_mode in modes {
                    for blur_mode in modes {
                        for spread_mode in modes {
                            let mut ctx = parser_context();
                            let value = json!({
                                "color": color_json(color_mode),
                                "offsetX": offset_x_json(offset_x_mode),
                                "offsetY": offset_y_json(offset_y_mode),
                                "blur": blur_json(blur_mode),
                                "spread": spread_json(spread_mode),
                            });

                            let parsed = ShadowTokenSingleValue::try_from_json(&mut ctx, "/token", &value)
                                .unwrap_or_else(|| {
                                    panic!(
                                        "expected shadow single to parse for combination {:?}/{:?}/{:?}/{:?}/{:?}; errors: {:?}",
                                        color_mode, offset_x_mode, offset_y_mode, blur_mode, spread_mode, ctx.errors
                                    )
                                });

                            assert_eq!(classify_color(&parsed.color), color_mode);
                            assert_eq!(classify_offset_x(&parsed.offset_x), offset_x_mode);
                            assert_eq!(classify_offset_y(&parsed.offset_y), offset_y_mode);
                            assert_eq!(classify_blur(&parsed.blur), blur_mode);
                            assert_eq!(classify_spread(&parsed.spread), spread_mode);
                            assert_eq!(parsed.inset, None);
                            assert!(ctx.errors.is_empty());
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn preserves_shadow_single_when_optional_inset_is_invalid() {
        let mut ctx = parser_context();
        let value = json!({
            "color": color_json(FieldMode::Literal),
            "offsetX": offset_x_json(FieldMode::Literal),
            "offsetY": offset_y_json(FieldMode::Literal),
            "blur": blur_json(FieldMode::Literal),
            "spread": spread_json(FieldMode::Literal),
            "inset": "true"
        });

        let parsed = ShadowTokenSingleValue::try_from_json(&mut ctx, "/token", &value);

        assert!(parsed.is_some());
        assert_eq!(parsed.unwrap().inset, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].path, "/token/inset");
    }

    #[test]
    fn reports_missing_required_shadow_single_fields() {
        let mut ctx = parser_context();

        let parsed = ShadowTokenSingleValue::try_from_json(&mut ctx, "/token", &json!({}));

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 6);
        assert_eq!(ctx.errors[0].path, "/token/color");
        assert_eq!(ctx.errors[1].path, "/token/offsetX");
        assert_eq!(ctx.errors[2].path, "/token/offsetY");
        assert_eq!(ctx.errors[3].path, "/token/blur");
        assert_eq!(ctx.errors[4].path, "/token/spread");
        assert_eq!(ctx.errors[5].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(ctx.errors[5].path, "/token");
    }

    #[test]
    fn parses_shadow_token_value_as_single_literal() {
        let mut ctx = parser_context();
        let value = json!({
            "color": color_json(FieldMode::Literal),
            "offsetX": offset_x_json(FieldMode::Literal),
            "offsetY": offset_y_json(FieldMode::Literal),
            "blur": blur_json(FieldMode::Literal),
            "spread": spread_json(FieldMode::Literal),
            "inset": false
        });

        let parsed = ShadowTokenValue::try_from_json(&mut ctx, "/token", &value);

        match parsed {
            Some(ShadowTokenValue::Single(RefAliasOrLiteral::Literal(single))) => {
                assert!(matches!(single.color, RefOr::Literal(ShadowColor(AliasOrLiteral::Literal(_)))));
                assert_eq!(single.inset, Some(RefOr::Literal(ShadowInset(false))));
            }
            _ => panic!("expected single literal shadow token"),
        }
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn parses_shadow_token_value_as_array_with_literal_alias_and_reference_items() {
        let mut ctx = parser_context();
        let value = json!([
            {
                "color": color_json(FieldMode::Literal),
                "offsetX": offset_x_json(FieldMode::Literal),
                "offsetY": offset_y_json(FieldMode::Literal),
                "blur": blur_json(FieldMode::Literal),
                "spread": spread_json(FieldMode::Literal)
            },
            "{shadow.elevation.sm}",
            { "$ref": "#/shadow/preset/lg" }
        ]);

        let parsed = ShadowTokenValue::try_from_json(&mut ctx, "/token", &value);

        match parsed {
            Some(ShadowTokenValue::Array(RefOr::Literal(items))) => {
                assert_eq!(items.len(), 3);
                assert!(matches!(items[0], RefAliasOrLiteral::Literal(_)));
                assert_eq!(
                    items[1],
                    RefAliasOrLiteral::Alias(TokenAlias::from_dtcg_alias("{shadow.elevation.sm}").unwrap())
                );
                assert_eq!(
                    items[2],
                    RefAliasOrLiteral::Ref(JsonRef::new_local_pointer(
                        "#/shadow/preset/lg".into(),
                        JsonPointer::from("#/shadow/preset/lg"),
                    ))
                );
            }
            _ => panic!("expected array shadow token"),
        }
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn rejects_shadow_array_when_any_item_is_invalid() {
        let mut ctx = parser_context();
        let value = json!([
            {
                "color": color_json(FieldMode::Literal),
                "offsetX": offset_x_json(FieldMode::Literal),
                "offsetY": offset_y_json(FieldMode::Literal),
                "blur": blur_json(FieldMode::Literal),
                "spread": spread_json(FieldMode::Literal)
            },
            true
        ]);

        let parsed = ShadowTokenValue::try_from_json(&mut ctx, "/token", &value);

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 2);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyType);
        assert_eq!(ctx.errors[0].path, "/token/1");
        assert_eq!(ctx.errors[1].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(ctx.errors[1].path, "/token/1");
    }

    #[test]
    fn rejects_invalid_top_level_shadow_token_type() {
        let mut ctx = parser_context();

        let parsed = ShadowTokenValue::try_from_json(&mut ctx, "/token", &json!(42));

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(ctx.errors[0].path, "/token");
    }
}