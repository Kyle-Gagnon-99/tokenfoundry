//! The `gradient` module defines the `GradientTokenValue` struct which represents the DTCG gradient token type.

use crate::{
    ParserContext,
    errors::DiagnosticCode,
    ir::{
        DiagnosticOwnership, InvalidReason, JsonNumber, JsonObject, ParseState, RefAliasOrLiteral,
        RefOrLiteral, TryFromJson,
    },
    token::token_types::color::ColorTokenValue,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GradientObjectColor(pub RefAliasOrLiteral<ColorTokenValue>);

impl<'a> TryFromJson<'a> for GradientObjectColor {
    fn try_from_json(
        ctx: &mut ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match RefAliasOrLiteral::<ColorTokenValue>::try_from_json(ctx, path, value) {
            ParseState::Parsed(val) => ParseState::Parsed(Self(val)),
            ParseState::Invalid(inv) => {
                if inv.ownership == DiagnosticOwnership::Silent {
                    ctx.push_to_errors(DiagnosticCode::InvalidPropertyValue, format!("Expected a DTCG reference, a JSON reference, or a color token, but got {:?}", value), path.into());
                    return ParseState::invalid_emitted(InvalidReason::InvalidValue);
                }
                ParseState::Invalid(inv)
            }
            ParseState::NoMatch => {
                ctx.push_to_errors(DiagnosticCode::InvalidPropertyValue, format!("Expected a DTCG reference, a JSON reference, or a color token, but got {:?}", value), path.into());
                ParseState::invalid_emitted(InvalidReason::InvalidValue)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GradientObjectPosition(pub RefOrLiteral<JsonNumber>);

impl<'a> TryFromJson<'a> for GradientObjectPosition {
    fn try_from_json(
        ctx: &mut ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match RefOrLiteral::<JsonNumber>::try_from_json(ctx, path, value) {
            ParseState::Parsed(val) => ParseState::Parsed(Self(val)),
            ParseState::Invalid(inv) => {
                if inv.ownership == DiagnosticOwnership::Silent {
                    ctx.push_to_errors(
                        DiagnosticCode::InvalidPropertyValue,
                        format!("Expected a JSON reference, or a number but got {:?}", value),
                        path.into(),
                    );
                    return ParseState::invalid_emitted(InvalidReason::InvalidValue);
                }
                ParseState::Invalid(inv)
            }
            ParseState::NoMatch => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidPropertyValue,
                    format!("Expected a JSON reference, or a number but got {:?}", value),
                    path.into(),
                );
                ParseState::invalid_emitted(InvalidReason::InvalidValue)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GradientObject {
    pub color: GradientObjectColor,
    pub position: GradientObjectPosition,
}

impl<'a> TryFromJson<'a> for GradientObject {
    fn try_from_json(
        ctx: &mut ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        let obj = match value {
            serde_json::Value::Object(map) => JsonObject(map),
            _ => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidPropertyValue,
                    format!("Expected a gradient object, but got {:?}", value),
                    path.into(),
                );
                return ParseState::invalid_emitted(InvalidReason::InvalidValue);
            }
        };

        let color = obj.required_field::<GradientObjectColor>(ctx, path, "color");
        let position = obj.required_field::<GradientObjectPosition>(ctx, path, "position");

        match (color, position) {
            (Some(color), Some(position)) => ParseState::Parsed(Self { color, position }),
            _ => ParseState::invalid_emitted(InvalidReason::Other),
        }
    }
}

impl<'a> TryFromJson<'a> for Vec<RefAliasOrLiteral<GradientObject>> {
    fn try_from_json(
        ctx: &mut ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match value {
            serde_json::Value::Array(arr) => {
                let mut result = Vec::new();
                for (index, item) in arr.iter().enumerate() {
                    match RefAliasOrLiteral::<GradientObject>::try_from_json(
                        ctx,
                        &format!("{}/{}", path, index),
                        item,
                    ) {
                        ParseState::Parsed(val) => result.push(val),
                        ParseState::Invalid(inv) => {
                            if inv.ownership == DiagnosticOwnership::Silent {
                                ctx.push_to_errors(
                                    DiagnosticCode::InvalidPropertyValue,
                                    format!("Expected a DTCG reference, a JSON reference, or a gradient object, but got {:?}", item),
                                    format!("{}/{}", path, index).into(),
                                );
                                return ParseState::invalid_emitted(InvalidReason::InvalidValue);
                            }
                            return ParseState::Invalid(inv);
                        }
                        ParseState::NoMatch => {
                            ctx.push_to_errors(
                                DiagnosticCode::InvalidPropertyValue,
                                format!("Expected a DTCG reference, a JSON reference, or a gradient object, but got {:?}", item),
                                format!("{}/{}", path, index).into(),
                            );
                            return ParseState::invalid_emitted(InvalidReason::InvalidValue);
                        }
                    }
                }
                ParseState::Parsed(result)
            }
            _ => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidPropertyValue,
                    format!(
                        "Expected an array of gradient objects or references, but got {:?}",
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
pub struct GradientTokenValue(pub RefOrLiteral<Vec<RefAliasOrLiteral<GradientObject>>>);

impl<'a> TryFromJson<'a> for GradientTokenValue {
    fn try_from_json(
        ctx: &mut ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match RefOrLiteral::<Vec<RefAliasOrLiteral<GradientObject>>>::try_from_json(
            ctx, path, value,
        ) {
            ParseState::Parsed(val) => ParseState::Parsed(Self(val)),
            ParseState::Invalid(inv) => {
                if inv.ownership == DiagnosticOwnership::Silent {
                    ctx.push_to_errors(
                        DiagnosticCode::InvalidPropertyValue,
                        format!("Expected a reference to an array of gradient objects or an array of gradient objects, but got {:?}", value),
                        path.into(),
                    );
                    return ParseState::invalid_emitted(InvalidReason::InvalidValue);
                }
                ParseState::Invalid(inv)
            }
            ParseState::NoMatch => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidPropertyValue,
                    format!("Expected a reference to an array of gradient objects or an array of gradient objects, but got {:?}", value),
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
            "components": [1, 0, 0]
        })
    }

    fn valid_gradient_object() -> serde_json::Value {
        json!({
            "color": valid_color_value(),
            "position": 0.25
        })
    }

    #[test]
    fn gradient_object_color_parses_literal_ref_and_alias() {
        let mut ctx = test_ctx();

        let literal =
            GradientObjectColor::try_from_json(&mut ctx, "#/token/color", &valid_color_value());
        let by_ref = GradientObjectColor::try_from_json(
            &mut ctx,
            "#/token/color",
            &json!({"$ref": "#/colors/brand"}),
        );
        let alias =
            GradientObjectColor::try_from_json(&mut ctx, "#/token/color", &json!("{colors.brand}"));

        assert!(matches!(
            literal,
            ParseState::Parsed(GradientObjectColor(RefAliasOrLiteral::Literal(_)))
        ));
        assert!(matches!(
            by_ref,
            ParseState::Parsed(GradientObjectColor(RefAliasOrLiteral::Ref(_)))
        ));
        assert!(matches!(
            alias,
            ParseState::Parsed(GradientObjectColor(RefAliasOrLiteral::Alias(_)))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn gradient_object_color_rejects_invalid_value() {
        let mut ctx = test_ctx();

        let state = GradientObjectColor::try_from_json(&mut ctx, "#/token/color", &json!(true));

        assert!(matches!(state, ParseState::Invalid(_)));
        assert!(ctx.errors.iter().any(|e| {
            e.code == DiagnosticCode::InvalidPropertyValue && e.path == "#/token/color"
        }));
    }

    #[test]
    fn gradient_object_position_parses_literal_and_ref() {
        let mut ctx = test_ctx();

        let literal =
            GradientObjectPosition::try_from_json(&mut ctx, "#/token/position", &json!(0.5));
        let by_ref = GradientObjectPosition::try_from_json(
            &mut ctx,
            "#/token/position",
            &json!({"$ref": "#/positions/center"}),
        );

        assert!(matches!(
            literal,
            ParseState::Parsed(GradientObjectPosition(RefOrLiteral::Literal(_)))
        ));
        assert!(matches!(
            by_ref,
            ParseState::Parsed(GradientObjectPosition(RefOrLiteral::Ref(_)))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn gradient_object_position_rejects_invalid_value() {
        let mut ctx = test_ctx();

        let state =
            GradientObjectPosition::try_from_json(&mut ctx, "#/token/position", &json!(false));

        assert!(matches!(state, ParseState::Invalid(_)));
        assert!(ctx.errors.iter().any(|e| {
            e.code == DiagnosticCode::InvalidPropertyType && e.path == "#/token/position"
        }));
    }

    #[test]
    fn gradient_object_parses_valid_object() {
        let mut ctx = test_ctx();

        let state = GradientObject::try_from_json(&mut ctx, "#/token/0", &valid_gradient_object());

        assert!(matches!(state, ParseState::Parsed(_)));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn gradient_object_reports_missing_required_fields() {
        let mut ctx = test_ctx();
        let input = json!({
            "color": valid_color_value()
        });

        let state = GradientObject::try_from_json(&mut ctx, "#/token/0", &input);

        assert!(matches!(state, ParseState::Invalid(_)));
        assert!(ctx.errors.iter().any(|e| {
            e.code == DiagnosticCode::MissingRequiredProperty && e.path == "#/token/0/position"
        }));
    }

    #[test]
    fn gradient_object_rejects_non_object() {
        let mut ctx = test_ctx();

        let state = GradientObject::try_from_json(&mut ctx, "#/token/0", &json!("bad"));

        assert!(matches!(state, ParseState::Invalid(_)));
        assert!(
            ctx.errors.iter().any(|e| {
                e.code == DiagnosticCode::InvalidPropertyValue && e.path == "#/token/0"
            })
        );
    }

    #[test]
    fn gradient_array_parses_literals_refs_and_aliases() {
        let mut ctx = test_ctx();
        let input = json!([
            valid_gradient_object(),
            {"$ref": "#/gradients/primary"},
            "{gradients.secondary}"
        ]);

        let state =
            Vec::<RefAliasOrLiteral<GradientObject>>::try_from_json(&mut ctx, "#/token", &input);

        let parsed = match state {
            ParseState::Parsed(v) => v,
            _ => panic!("expected parsed gradient array"),
        };

        assert_eq!(parsed.len(), 3);
        assert!(matches!(parsed[0], RefAliasOrLiteral::Literal(_)));
        assert!(matches!(parsed[1], RefAliasOrLiteral::Ref(_)));
        assert!(matches!(parsed[2], RefAliasOrLiteral::Alias(_)));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn gradient_array_rejects_non_array() {
        let mut ctx = test_ctx();

        let state = Vec::<RefAliasOrLiteral<GradientObject>>::try_from_json(
            &mut ctx,
            "#/token",
            &json!({"color": "red"}),
        );

        assert!(matches!(state, ParseState::Invalid(_)));
        assert!(
            ctx.errors
                .iter()
                .any(|e| { e.code == DiagnosticCode::InvalidPropertyValue && e.path == "#/token" })
        );
    }

    #[test]
    fn gradient_array_rejects_invalid_entry() {
        let mut ctx = test_ctx();
        let input = json!([true]);

        let state =
            Vec::<RefAliasOrLiteral<GradientObject>>::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::Invalid(_)));
        assert!(
            ctx.errors.iter().any(|e| {
                e.code == DiagnosticCode::InvalidPropertyValue && e.path == "#/token/0"
            })
        );
    }

    #[test]
    fn gradient_token_value_parses_literal_array() {
        let mut ctx = test_ctx();
        let input = json!([valid_gradient_object()]);

        let state = GradientTokenValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(
            state,
            ParseState::Parsed(GradientTokenValue(RefOrLiteral::Literal(_)))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn gradient_token_value_parses_top_level_reference() {
        let mut ctx = test_ctx();

        let state = GradientTokenValue::try_from_json(
            &mut ctx,
            "#/token",
            &json!({"$ref": "#/gradients/primary"}),
        );

        assert!(matches!(
            state,
            ParseState::Parsed(GradientTokenValue(RefOrLiteral::Ref(_)))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn gradient_token_value_rejects_invalid_shape() {
        let mut ctx = test_ctx();

        let state = GradientTokenValue::try_from_json(&mut ctx, "#/token", &json!(123));

        assert!(matches!(state, ParseState::Invalid(_)));
        assert!(
            ctx.errors
                .iter()
                .any(|e| { e.code == DiagnosticCode::InvalidPropertyValue && e.path == "#/token" })
        );
    }
}
