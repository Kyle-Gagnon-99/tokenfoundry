//! The `shadow` module defines the `ShadowTokenValue` struct which represents the DTCG shadow token type.

use crate::{
    errors::DiagnosticCode,
    ir::token::token_types::{color::ColorTokenValue, dimension::DimensionTokenValue},
    ir::{
        DiagnosticEmission, InvalidReason, JsonObject, ParseState, RefAliasOrLiteral, RefOrLiteral,
        TryFromJson,
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShadowObjectColor(pub RefAliasOrLiteral<ColorTokenValue>);

impl<'a> TryFromJson<'a> for ShadowObjectColor {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &JsonPointer,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match RefAliasOrLiteral::<ColorTokenValue>::try_from_json(ctx, path, value) {
            ParseState::Parsed(val) => ParseState::Parsed(Self(val)),
            ParseState::Failed(inv) => {
                if inv.ownership == DiagnosticEmission::Silent {
                    ctx.push_to_errors(DiagnosticCode::InvalidPropertyValue, format!("Expected a DTCG reference, JSON reference, or a color token but got {:?}", value), path.into());
                    return ParseState::invalid_emitted(InvalidReason::InvalidValue);
                }
                ParseState::Failed(inv)
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShadowObjectOffsetX(pub RefAliasOrLiteral<DimensionTokenValue>);

impl<'a> TryFromJson<'a> for ShadowObjectOffsetX {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &JsonPointer,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match RefAliasOrLiteral::<DimensionTokenValue>::try_from_json(ctx, path, value) {
            ParseState::Parsed(val) => ParseState::Parsed(Self(val)),
            ParseState::Failed(inv) => {
                if inv.ownership == DiagnosticEmission::Silent {
                    ctx.push_to_errors(DiagnosticCode::InvalidPropertyValue, format!("Expected a DTCG reference, JSON reference, or a dimension token but got {:?}", value), path.into());
                    return ParseState::invalid_emitted(InvalidReason::InvalidValue);
                }
                ParseState::Failed(inv)
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShadowObjectOffsetY(pub RefAliasOrLiteral<DimensionTokenValue>);

impl<'a> TryFromJson<'a> for ShadowObjectOffsetY {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &JsonPointer,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match RefAliasOrLiteral::<DimensionTokenValue>::try_from_json(ctx, path, value) {
            ParseState::Parsed(val) => ParseState::Parsed(Self(val)),
            ParseState::Failed(inv) => {
                if inv.ownership == DiagnosticEmission::Silent {
                    ctx.push_to_errors(DiagnosticCode::InvalidPropertyValue, format!("Expected a DTCG reference, JSON reference, or a dimension token but got {:?}", value), path.into());
                    return ParseState::invalid_emitted(InvalidReason::InvalidValue);
                }
                ParseState::Failed(inv)
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShadowObjectBlur(pub RefAliasOrLiteral<DimensionTokenValue>);

impl<'a> TryFromJson<'a> for ShadowObjectBlur {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &JsonPointer,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match RefAliasOrLiteral::<DimensionTokenValue>::try_from_json(ctx, path, value) {
            ParseState::Parsed(val) => ParseState::Parsed(Self(val)),
            ParseState::Failed(inv) => {
                if inv.ownership == DiagnosticEmission::Silent {
                    ctx.push_to_errors(DiagnosticCode::InvalidPropertyValue, format!("Expected a DTCG reference, JSON reference, or a dimension token but got {:?}", value), path.into());
                    return ParseState::invalid_emitted(InvalidReason::InvalidValue);
                }
                ParseState::Failed(inv)
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShadowObjectSpread(pub RefAliasOrLiteral<DimensionTokenValue>);

impl<'a> TryFromJson<'a> for ShadowObjectSpread {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &JsonPointer,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match RefAliasOrLiteral::<DimensionTokenValue>::try_from_json(ctx, path, value) {
            ParseState::Parsed(val) => ParseState::Parsed(Self(val)),
            ParseState::Failed(inv) => {
                if inv.ownership == DiagnosticEmission::Silent {
                    ctx.push_to_errors(DiagnosticCode::InvalidPropertyValue, format!("Expected a DTCG reference, JSON reference, or a dimension token but got {:?}", value), path.into());
                    return ParseState::invalid_emitted(InvalidReason::InvalidValue);
                }
                ParseState::Failed(inv)
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

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShadowObjectInset(pub RefOrLiteral<bool>);

impl<'a> TryFromJson<'a> for ShadowObjectInset {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &JsonPointer,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match RefOrLiteral::<bool>::try_from_json(ctx, path, value) {
            ParseState::Parsed(val) => ParseState::Parsed(Self(val)),
            ParseState::Failed(inv) => {
                if inv.ownership == DiagnosticEmission::Silent {
                    ctx.push_to_errors(DiagnosticCode::InvalidPropertyValue, format!("Expected a DTCG reference, JSON reference, or a boolean token but got {:?}", value), path.into());
                    return ParseState::invalid_emitted(InvalidReason::InvalidValue);
                }
                ParseState::Failed(inv)
            }
            ParseState::NoMatch => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidPropertyValue,
                    format!(
                        "Expected a DTCG reference, JSON reference, or a boolean token but got {:?}",
                        value
                    ),
                    path.into(),
                );
                ParseState::invalid_emitted(InvalidReason::InvalidValue)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShadowObjectValue {
    pub color: ShadowObjectColor,
    pub offset_x: ShadowObjectOffsetX,
    pub offset_y: ShadowObjectOffsetY,
    pub blur: ShadowObjectBlur,
    pub spread: ShadowObjectSpread,
    pub inset: Option<ShadowObjectInset>,
}

impl<'a> TryFromJson<'a> for ShadowObjectValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &JsonPointer,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        let obj = match value {
            serde_json::Value::Object(map) => JsonObject::new(map),
            _ => return ParseState::NoMatch,
        };

        // For each required field, if it's missing the required_field method will add an error to the context and return ParseState::NoMatch
        let color = obj.required_field::<ShadowObjectColor>(ctx, path, "color");
        let offset_x = obj.required_field::<ShadowObjectOffsetX>(ctx, path, "offsetX");
        let offset_y = obj.required_field::<ShadowObjectOffsetY>(ctx, path, "offsetY");
        let blur = obj.required_field::<ShadowObjectBlur>(ctx, path, "blur");
        let spread = obj.required_field::<ShadowObjectSpread>(ctx, path, "spread");
        let inset = obj.optional_field::<ShadowObjectInset>(ctx, path, "inset");

        match (color, offset_x, offset_y, blur, spread) {
            (Some(color), Some(offset_x), Some(offset_y), Some(blur), Some(spread)) => {
                ParseState::Parsed(Self {
                    color,
                    offset_x,
                    offset_y,
                    blur,
                    spread,
                    inset,
                })
            }
            _ => ParseState::invalid_silent(InvalidReason::InvalidValue),
        }
    }
}

impl<'a> TryFromJson<'a> for Vec<RefAliasOrLiteral<ShadowObjectValue>> {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &JsonPointer,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match value {
            serde_json::Value::Array(arr) => {
                let mut result = Vec::new();
                for (index, item) in arr.iter().enumerate() {
                    match RefAliasOrLiteral::<ShadowObjectValue>::try_from_json(
                        ctx,
                        &format!("{}/{}", path, index),
                        item,
                    ) {
                        ParseState::Parsed(val) => result.push(val),
                        ParseState::Failed(inv) => return ParseState::Failed(inv),
                        ParseState::NoMatch => {
                            ctx.push_to_errors(
                                DiagnosticCode::InvalidPropertyValue,
                                format!(
                                    "Expected a DTCG reference, JSON reference, or a shadow object but got {:?}",
                                    item
                                ),
                                format!("{}/{}", path, index).into(),
                            );
                            return ParseState::invalid_emitted(InvalidReason::InvalidValue);
                        }
                    }
                }
                ParseState::Parsed(result)
            }
            _ => ParseState::NoMatch,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ShadowTokenValue(pub RefOrLiteral<Vec<RefAliasOrLiteral<ShadowObjectValue>>>);

impl<'a> TryFromJson<'a> for ShadowTokenValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &JsonPointer,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match RefOrLiteral::<Vec<RefAliasOrLiteral<ShadowObjectValue>>>::try_from_json(
            ctx, path, value,
        ) {
            ParseState::Parsed(val) => ParseState::Parsed(Self(val)),
            ParseState::Failed(inv) => {
                if inv.ownership == DiagnosticEmission::Silent {
                    ctx.push_to_errors(
                        DiagnosticCode::InvalidPropertyValue,
                        format!(
                            "Expected a DTCG reference, JSON reference, or an array of shadow objects but got {:?}",
                            value
                        ),
                        path.into(),
                    );
                    return ParseState::invalid_emitted(InvalidReason::InvalidValue);
                }
                ParseState::Failed(inv)
            }
            ParseState::NoMatch => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidPropertyValue,
                    format!(
                        "Expected a DTCG reference, JSON reference, or an array of shadow objects but got {:?}",
                        value
                    ),
                    path.into(),
                );
                ParseState::invalid_emitted(InvalidReason::InvalidValue)
            }
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
        ir::{ParseState, RefAliasOrLiteral, RefOrLiteral, TryFromJson},
    };

    fn test_ctx() -> ParserContext {
        ParserContext::new("{}".to_string())
    }

    fn valid_color_value() -> serde_json::Value {
        json!({
            "colorSpace": "srgb",
            "components": [0, 0, 0]
        })
    }

    fn valid_dimension_value(value: i64) -> serde_json::Value {
        json!({
            "value": value,
            "unit": "px"
        })
    }

    fn valid_shadow_object() -> serde_json::Value {
        json!({
            "color": valid_color_value(),
            "offsetX": valid_dimension_value(1),
            "offsetY": valid_dimension_value(2),
            "blur": valid_dimension_value(4),
            "spread": valid_dimension_value(0),
            "inset": false
        })
    }

    #[test]
    fn shadow_object_color_parses_literal_ref_and_alias() {
        let mut ctx = test_ctx();

        let literal =
            ShadowObjectColor::try_from_json(&mut ctx, "#/token/color", &valid_color_value());
        let by_ref = ShadowObjectColor::try_from_json(
            &mut ctx,
            "#/token/color",
            &json!({"$ref": "#/colors/shadow"}),
        );
        let alias =
            ShadowObjectColor::try_from_json(&mut ctx, "#/token/color", &json!("{colors.shadow}"));

        assert!(matches!(
            literal,
            ParseState::Parsed(ShadowObjectColor(RefAliasOrLiteral::Literal(_)))
        ));
        assert!(matches!(
            by_ref,
            ParseState::Parsed(ShadowObjectColor(RefAliasOrLiteral::Ref(_)))
        ));
        assert!(matches!(
            alias,
            ParseState::Parsed(ShadowObjectColor(RefAliasOrLiteral::Alias(_)))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn shadow_object_offset_x_parses_literal_ref_and_alias() {
        let mut ctx = test_ctx();

        let literal = ShadowObjectOffsetX::try_from_json(
            &mut ctx,
            "#/token/offsetX",
            &valid_dimension_value(1),
        );
        let by_ref = ShadowObjectOffsetX::try_from_json(
            &mut ctx,
            "#/token/offsetX",
            &json!({"$ref": "#/sizes/shadow-offset-x"}),
        );
        let alias = ShadowObjectOffsetX::try_from_json(
            &mut ctx,
            "#/token/offsetX",
            &json!("{sizes.shadow.offsetX}"),
        );

        assert!(matches!(
            literal,
            ParseState::Parsed(ShadowObjectOffsetX(RefAliasOrLiteral::Literal(_)))
        ));
        assert!(matches!(
            by_ref,
            ParseState::Parsed(ShadowObjectOffsetX(RefAliasOrLiteral::Ref(_)))
        ));
        assert!(matches!(
            alias,
            ParseState::Parsed(ShadowObjectOffsetX(RefAliasOrLiteral::Alias(_)))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn shadow_object_inset_parses_literal_and_reference() {
        let mut ctx = test_ctx();

        let literal = ShadowObjectInset::try_from_json(&mut ctx, "#/token/inset", &json!(true));
        let by_ref = ShadowObjectInset::try_from_json(
            &mut ctx,
            "#/token/inset",
            &json!({"$ref": "#/flags/shadow-inset"}),
        );

        assert!(matches!(
            literal,
            ParseState::Parsed(ShadowObjectInset(RefOrLiteral::Literal(true)))
        ));
        assert!(matches!(
            by_ref,
            ParseState::Parsed(ShadowObjectInset(RefOrLiteral::Ref(_)))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn shadow_object_value_parses_valid_object_with_inset() {
        let mut ctx = test_ctx();

        let state = ShadowObjectValue::try_from_json(&mut ctx, "#/token/0", &valid_shadow_object());

        assert!(matches!(state, ParseState::Parsed(_)));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn shadow_object_value_parses_without_optional_inset() {
        let mut ctx = test_ctx();
        let input = json!({
            "color": valid_color_value(),
            "offsetX": valid_dimension_value(1),
            "offsetY": valid_dimension_value(2),
            "blur": valid_dimension_value(4),
            "spread": valid_dimension_value(0)
        });

        let state = ShadowObjectValue::try_from_json(&mut ctx, "#/token/0", &input);

        let parsed = match state {
            ParseState::Parsed(v) => v,
            _ => panic!("expected parsed shadow object"),
        };
        assert!(parsed.inset.is_none());
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn shadow_object_value_reports_missing_required_field() {
        let mut ctx = test_ctx();
        let input = json!({
            "color": valid_color_value(),
            "offsetX": valid_dimension_value(1),
            "offsetY": valid_dimension_value(2),
            "blur": valid_dimension_value(4)
        });

        let state = ShadowObjectValue::try_from_json(&mut ctx, "#/token/0", &input);

        assert!(matches!(state, ParseState::Failed(_)));
        assert!(ctx.errors.iter().any(|e| {
            e.code == DiagnosticCode::MissingRequiredProperty && e.path == "#/token/0/spread"
        }));
    }

    #[test]
    fn shadow_object_value_returns_no_match_for_non_object() {
        let mut ctx = test_ctx();

        let state = ShadowObjectValue::try_from_json(&mut ctx, "#/token/0", &json!(42));

        assert!(matches!(state, ParseState::NoMatch));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn shadow_object_value_reports_invalid_optional_inset_type_but_still_parses() {
        let mut ctx = test_ctx();
        let input = json!({
            "color": valid_color_value(),
            "offsetX": valid_dimension_value(1),
            "offsetY": valid_dimension_value(2),
            "blur": valid_dimension_value(4),
            "spread": valid_dimension_value(0),
            "inset": "sometimes"
        });

        let state = ShadowObjectValue::try_from_json(&mut ctx, "#/token/0", &input);

        let parsed = match state {
            ParseState::Parsed(v) => v,
            _ => panic!("expected parsed shadow object"),
        };
        assert!(parsed.inset.is_none());
        assert!(ctx.errors.iter().any(|e| e.path == "#/token/0/inset"));
    }

    #[test]
    fn shadow_array_parses_literals_refs_and_aliases() {
        let mut ctx = test_ctx();
        let input = json!([
            valid_shadow_object(),
            {"$ref": "#/shadows/elevation-sm"},
            "{shadows.elevation.md}"
        ]);

        let state =
            Vec::<RefAliasOrLiteral<ShadowObjectValue>>::try_from_json(&mut ctx, "#/token", &input);

        let parsed = match state {
            ParseState::Parsed(v) => v,
            _ => panic!("expected parsed shadow array"),
        };

        assert_eq!(parsed.len(), 3);
        assert!(matches!(parsed[0], RefAliasOrLiteral::Literal(_)));
        assert!(matches!(parsed[1], RefAliasOrLiteral::Ref(_)));
        assert!(matches!(parsed[2], RefAliasOrLiteral::Alias(_)));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn shadow_array_returns_no_match_for_non_array() {
        let mut ctx = test_ctx();

        let state = Vec::<RefAliasOrLiteral<ShadowObjectValue>>::try_from_json(
            &mut ctx,
            "#/token",
            &json!({}),
        );

        assert!(matches!(state, ParseState::NoMatch));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn shadow_array_reports_invalid_entry() {
        let mut ctx = test_ctx();
        let input = json!([true]);

        let state =
            Vec::<RefAliasOrLiteral<ShadowObjectValue>>::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::Failed(_)));
        assert!(ctx.errors.iter().any(|e| e.path == "#/token/0"));
    }

    #[test]
    fn shadow_token_value_parses_literal_array() {
        let mut ctx = test_ctx();
        let input = json!([valid_shadow_object()]);

        let state = ShadowTokenValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(
            state,
            ParseState::Parsed(ShadowTokenValue(RefOrLiteral::Literal(_)))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn shadow_token_value_parses_top_level_reference() {
        let mut ctx = test_ctx();

        let state = ShadowTokenValue::try_from_json(
            &mut ctx,
            "#/token",
            &json!({"$ref": "#/shadows/elevation-lg"}),
        );

        assert!(matches!(
            state,
            ParseState::Parsed(ShadowTokenValue(RefOrLiteral::Ref(_)))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn shadow_token_value_rejects_invalid_shape() {
        let mut ctx = test_ctx();

        let state = ShadowTokenValue::try_from_json(&mut ctx, "#/token", &json!(123));

        assert!(matches!(state, ParseState::Failed(_)));
        assert!(
            ctx.errors
                .iter()
                .any(|e| { e.code == DiagnosticCode::InvalidPropertyValue && e.path == "#/token" })
        );
    }
}
