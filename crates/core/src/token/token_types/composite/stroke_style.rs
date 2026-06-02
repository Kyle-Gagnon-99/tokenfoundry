//! The `stroke_style` module defines the `StrokeStyleTokenValue` struct which represents the DTCG stroke-style token type.

use crate::{
    errors::DiagnosticCode,
    ir::{
        DiagnosticOwnership, InvalidReason, JsonArray, JsonObject, ParseState, RefAliasOrLiteral,
        RefOrLiteral, TryFromJson,
    },
    token::token_types::dimension::DimensionTokenValue,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrokeStyleStringValue {
    Solid,
    Dashed,
    Dotted,
    Double,
    Groove,
    Ridge,
    Outset,
    Inset,
}

impl<'a> TryFromJson<'a> for StrokeStyleStringValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> crate::ir::ParseState<Self> {
        match value {
            serde_json::Value::String(s) => match s.as_str() {
                "solid" => ParseState::Parsed(Self::Solid),
                "dashed" => ParseState::Parsed(Self::Dashed),
                "dotted" => ParseState::Parsed(Self::Dotted),
                "double" => ParseState::Parsed(Self::Double),
                "groove" => ParseState::Parsed(Self::Groove),
                "ridge" => ParseState::Parsed(Self::Ridge),
                "outset" => ParseState::Parsed(Self::Outset),
                "inset" => ParseState::Parsed(Self::Inset),
                _ => {
                    ctx.push_to_errors(
                        DiagnosticCode::InvalidPropertyValue,
                        format!("Invalid stroke-style value '{}' at {}", s, path),
                        path.into(),
                    );
                    ParseState::invalid_emitted(InvalidReason::InvalidFieldType)
                }
            },
            _ => ParseState::NoMatch,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrokeStyleLineCapValue {
    Round,
    Butt,
    Square,
}

impl<'a> TryFromJson<'a> for StrokeStyleLineCapValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match value {
            serde_json::Value::String(s) => match s.as_str() {
                "round" => ParseState::Parsed(Self::Round),
                "butt" => ParseState::Parsed(Self::Butt),
                "square" => ParseState::Parsed(Self::Square),
                _ => {
                    ctx.push_to_errors(
                        DiagnosticCode::InvalidPropertyValue,
                        format!("Invalid stroke-style line-cap value '{}' at {}", s, path),
                        path.into(),
                    );
                    ParseState::invalid_emitted(InvalidReason::InvalidFieldType)
                }
            },
            _ => ParseState::NoMatch,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrokeStyleDashArrayValue(pub Vec<RefAliasOrLiteral<DimensionTokenValue>>);

impl<'a> TryFromJson<'a> for StrokeStyleDashArrayValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match value {
            serde_json::Value::Array(arr) => {
                let arr_val = JsonArray::new(arr);
                let result = match arr_val
                    .parse_for_each::<RefAliasOrLiteral<DimensionTokenValue>>(ctx, path)
                {
                    Some(val) => val,
                    None => {
                        ctx.push_to_errors(
                            DiagnosticCode::InvalidPropertyValue,
                            format!("Expected all entries to be either dimension values or references to dimension values for stroke-style dash-array token at {}", path),
                            path.into(),
                        );
                        return ParseState::invalid_emitted(InvalidReason::InvalidFieldType);
                    }
                };

                if result.is_empty() {
                    ctx.push_to_errors(
                        DiagnosticCode::InvalidPropertyValue,
                        format!("Expected at least one dash length for stroke-style dash-array token at {}", path),
                        path.into(),
                    );
                    return ParseState::invalid_emitted(InvalidReason::InvalidFieldType);
                }

                ParseState::Parsed(Self(result))
            }
            _ => ParseState::NoMatch,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrokeStyleObjectValue {
    pub dash_array: RefOrLiteral<StrokeStyleDashArrayValue>,
    pub line_cap: RefOrLiteral<StrokeStyleLineCapValue>,
}

impl<'a> TryFromJson<'a> for StrokeStyleObjectValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        let obj = match value {
            serde_json::Value::Object(map) => JsonObject::new(map),
            _ => return ParseState::NoMatch,
        };

        // Parse the dash-array property
        let dash_array =
            obj.required_field::<RefOrLiteral<StrokeStyleDashArrayValue>>(ctx, path, "dashArray");

        let line_cap =
            obj.required_field::<RefOrLiteral<StrokeStyleLineCapValue>>(ctx, path, "lineCap");

        match (dash_array, line_cap) {
            (Some(dash_array), Some(line_cap)) => ParseState::Parsed(Self {
                dash_array,
                line_cap,
            }),
            _ => ParseState::invalid_silent(InvalidReason::Other),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StrokeStyleTokenValue {
    String(RefOrLiteral<StrokeStyleStringValue>),
    Object(RefOrLiteral<StrokeStyleObjectValue>),
}

impl<'a> TryFromJson<'a> for StrokeStyleTokenValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match RefOrLiteral::<StrokeStyleStringValue>::try_from_json(ctx, path, value) {
            ParseState::Parsed(val) => ParseState::Parsed(Self::String(val)),
            ParseState::Invalid(inv) => {
                if inv.ownership == DiagnosticOwnership::Silent {
                    ctx.push_to_errors(
                        DiagnosticCode::InvalidTokenValue,
                        format!("Expected either a string value or an object with 'dashArray' and 'lineCap' properties for stroke-style token at {}, but got {:?}", path, value),
                        path.into(),
                    );
                    return ParseState::invalid_emitted(InvalidReason::InvalidValue);
                }
                ParseState::Invalid(inv)
            }
            ParseState::NoMatch => {
                match RefOrLiteral::<StrokeStyleObjectValue>::try_from_json(ctx, path, value) {
                    ParseState::Parsed(obj_val) => ParseState::Parsed(Self::Object(obj_val)),
                    ParseState::Invalid(inv) => {
                        if inv.ownership == DiagnosticOwnership::Silent {
                            ctx.push_to_errors(
                                DiagnosticCode::InvalidTokenValue,
                                format!("Expected either a string value or an object with 'dashArray' and 'lineCap' properties for stroke-style token at {}, but got {:?}", path, value),
                                path.into(),
                            );
                            return ParseState::invalid_emitted(InvalidReason::InvalidValue);
                        }
                        ParseState::Invalid(inv)
                    }
                    ParseState::NoMatch => {
                        ctx.push_to_errors(
                            DiagnosticCode::InvalidTokenValue,
                            format!("Expected either a string value or an object with 'dashArray' and 'lineCap' properties for stroke-style token at {}, but got {:?}", path, value),
                            path.into(),
                        );
                        ParseState::invalid_emitted(InvalidReason::InvalidValue)
                    }
                }
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

    #[test]
    fn stroke_style_string_value_parses_supported_values() {
        let mut ctx = test_ctx();

        let solid = StrokeStyleStringValue::try_from_json(&mut ctx, "#/token", &json!("solid"));
        let dotted = StrokeStyleStringValue::try_from_json(&mut ctx, "#/token", &json!("dotted"));

        assert!(matches!(
            solid,
            ParseState::Parsed(StrokeStyleStringValue::Solid)
        ));
        assert!(matches!(
            dotted,
            ParseState::Parsed(StrokeStyleStringValue::Dotted)
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn stroke_style_string_value_rejects_invalid_string() {
        let mut ctx = test_ctx();

        let state = StrokeStyleStringValue::try_from_json(&mut ctx, "#/token", &json!("wave"));

        assert!(matches!(state, ParseState::Invalid(_)));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyValue);
        assert_eq!(ctx.errors[0].path, "#/token");
    }

    #[test]
    fn stroke_style_line_cap_parses_supported_values() {
        let mut ctx = test_ctx();

        let round = StrokeStyleLineCapValue::try_from_json(&mut ctx, "#/token", &json!("round"));
        let butt = StrokeStyleLineCapValue::try_from_json(&mut ctx, "#/token", &json!("butt"));

        assert!(matches!(
            round,
            ParseState::Parsed(StrokeStyleLineCapValue::Round)
        ));
        assert!(matches!(
            butt,
            ParseState::Parsed(StrokeStyleLineCapValue::Butt)
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn stroke_style_line_cap_rejects_invalid_string() {
        let mut ctx = test_ctx();

        let state = StrokeStyleLineCapValue::try_from_json(&mut ctx, "#/token", &json!("flat"));

        assert!(matches!(state, ParseState::Invalid(_)));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyValue);
        assert_eq!(ctx.errors[0].path, "#/token");
    }

    #[test]
    fn stroke_style_dash_array_parses_dimension_literals_refs_and_aliases() {
        let mut ctx = test_ctx();
        let input = json!([
            { "value": 1, "unit": "px" },
            { "$ref": "#/tokens/dash/md" },
            "{spacing.xs}"
        ]);

        let state = StrokeStyleDashArrayValue::try_from_json(&mut ctx, "#/token/dashArray", &input);

        let parsed = match state {
            ParseState::Parsed(v) => v,
            _ => panic!("expected parsed stroke style dash array"),
        };

        assert_eq!(parsed.0.len(), 3);
        assert!(matches!(parsed.0[0], RefAliasOrLiteral::Literal(_)));
        assert!(matches!(parsed.0[1], RefAliasOrLiteral::Ref(_)));
        assert!(matches!(parsed.0[2], RefAliasOrLiteral::Alias(_)));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn stroke_style_dash_array_rejects_invalid_entry() {
        let mut ctx = test_ctx();
        let input = json!(["bad"]);

        let state = StrokeStyleDashArrayValue::try_from_json(&mut ctx, "#/token/dashArray", &input);

        assert!(matches!(state, ParseState::Invalid(_)));
        assert!(ctx.errors.iter().any(|e| {
            e.code == DiagnosticCode::InvalidPropertyType && e.path == "#/token/dashArray/0"
        }));
        assert!(ctx.errors.iter().any(|e| {
            e.code == DiagnosticCode::InvalidPropertyValue && e.path == "#/token/dashArray"
        }));
    }

    #[test]
    fn stroke_style_object_value_parses_valid_object() {
        let mut ctx = test_ctx();
        let input = json!({
            "dashArray": [ { "value": 1, "unit": "px" }, "{spacing.xs}" ],
            "lineCap": "round"
        });

        let state = StrokeStyleObjectValue::try_from_json(&mut ctx, "#/token", &input);

        let parsed = match state {
            ParseState::Parsed(v) => v,
            _ => panic!("expected parsed stroke style object value"),
        };

        assert!(matches!(parsed.dash_array, RefOrLiteral::Literal(_)));
        assert!(matches!(
            parsed.line_cap,
            RefOrLiteral::Literal(StrokeStyleLineCapValue::Round)
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn stroke_style_object_value_reports_missing_required_fields() {
        let mut ctx = test_ctx();
        let input = json!({ "dashArray": [ { "value": 1, "unit": "px" } ] });

        let state = StrokeStyleObjectValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::Invalid(_)));
        assert!(ctx.errors.iter().any(|e| {
            e.code == DiagnosticCode::MissingRequiredProperty && e.path == "#/token/lineCap"
        }));
    }

    #[test]
    fn stroke_style_object_value_reports_invalid_dash_array_type() {
        let mut ctx = test_ctx();
        let input = json!({
            "dashArray": 123,
            "lineCap": "round"
        });

        let state = StrokeStyleObjectValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::Invalid(_)));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn stroke_style_object_value_reports_invalid_line_cap_type() {
        let mut ctx = test_ctx();
        let input = json!({
            "dashArray": [ { "value": 1, "unit": "px" } ],
            "lineCap": true
        });

        let state = StrokeStyleObjectValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::Invalid(_)));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn stroke_style_dash_array_rejects_empty_array() {
        let mut ctx = test_ctx();
        let input = json!([]);

        let state = StrokeStyleDashArrayValue::try_from_json(&mut ctx, "#/token/dashArray", &input);

        assert!(matches!(state, ParseState::Invalid(_)));
        assert!(ctx.errors.iter().any(|e| {
            e.code == DiagnosticCode::InvalidPropertyValue && e.path == "#/token/dashArray"
        }));
    }

    #[test]
    fn stroke_style_object_value_accepts_ref_for_line_cap_and_dash_array() {
        let mut ctx = test_ctx();
        let input = json!({
            "dashArray": { "$ref": "#/tokens/dash-array" },
            "lineCap": { "$ref": "#/tokens/line-cap" }
        });

        let state = StrokeStyleObjectValue::try_from_json(&mut ctx, "#/token", &input);

        let parsed = match state {
            ParseState::Parsed(v) => v,
            _ => panic!("expected parsed stroke style object value"),
        };

        assert!(matches!(parsed.dash_array, RefOrLiteral::Ref(_)));
        assert!(matches!(parsed.line_cap, RefOrLiteral::Ref(_)));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn stroke_style_token_value_parses_string_variant() {
        let mut ctx = test_ctx();

        let state = StrokeStyleTokenValue::try_from_json(&mut ctx, "#/token", &json!("dashed"));

        assert!(matches!(
            state,
            ParseState::Parsed(StrokeStyleTokenValue::String(RefOrLiteral::Literal(
                StrokeStyleStringValue::Dashed
            )))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn stroke_style_token_value_parses_object_variant() {
        let mut ctx = test_ctx();
        let input = json!({
            "dashArray": [ { "value": 1, "unit": "px" } ],
            "lineCap": "square"
        });

        let state = StrokeStyleTokenValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(
            state,
            ParseState::Parsed(StrokeStyleTokenValue::Object(RefOrLiteral::Literal(_)))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn stroke_style_token_value_reports_invalid_token_value() {
        let mut ctx = test_ctx();

        let state = StrokeStyleTokenValue::try_from_json(&mut ctx, "#/token", &json!(42));

        assert!(matches!(state, ParseState::Invalid(_)));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(ctx.errors[0].path, "#/token");
    }
}
