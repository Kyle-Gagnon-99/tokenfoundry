//! The `dimension` module defines the `DimensionTokenValue` struct which represents the DTCG dimension token type

use crate::{
    errors::DiagnosticCode,
    ir::{JsonNumber, JsonObject, ParseState, RefOrLiteral, TryFromJson},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DimensionValue(pub JsonNumber);

impl<'a> TryFromJson<'a> for DimensionValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match JsonNumber::try_from_json(ctx, path, value) {
            ParseState::Parsed(number) => ParseState::Parsed(Self(number)),
            ParseState::Invalid => ParseState::Invalid,
            ParseState::NoMatch => ParseState::NoMatch,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DimensionUnit {
    Px,
    Rem,
}

impl<'a> TryFromJson<'a> for DimensionUnit {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match value {
            serde_json::Value::String(s) if s == "px" => ParseState::Parsed(Self::Px),
            serde_json::Value::String(s) if s == "rem" => ParseState::Parsed(Self::Rem),
            _ => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidPropertyValue,
                    format!("Expected either 'px' or 'rem' for unit at {}", path),
                    path.into(),
                );
                ParseState::Invalid
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DimensionTokenValue {
    pub value: RefOrLiteral<DimensionValue>,
    pub unit: RefOrLiteral<DimensionUnit>,
}

impl<'a> TryFromJson<'a> for DimensionTokenValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        let obj = match value {
            serde_json::Value::Object(map) => JsonObject::new(map),
            _ => {
                ctx.push_to_errors(
                    crate::errors::DiagnosticCode::InvalidPropertyType,
                    format!("Expected an object for dimension token value, but found: {value}"),
                    path.into(),
                );
                return ParseState::Invalid;
            }
        };

        // Now, require the "value" property
        let value = obj.required_field::<RefOrLiteral<DimensionValue>>(ctx, path, "value");
        let unit = obj.required_field::<RefOrLiteral<DimensionUnit>>(ctx, path, "unit");

        match (value, unit) {
            (ParseState::Parsed(value), ParseState::Parsed(unit)) => {
                ParseState::Parsed(Self { value, unit })
            }
            _ => ParseState::Invalid,
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{
        FileFormat, ParserContext,
        errors::DiagnosticCode,
        ir::{JsonRefObject, ParseState, TryFromJson},
    };

    fn test_ctx() -> ParserContext {
        ParserContext::new("test.json".to_string(), FileFormat::Json, "{}".to_string())
    }

    #[test]
    fn dimension_unit_parses_supported_units() {
        let mut ctx = test_ctx();

        let px_state = DimensionUnit::try_from_json(&mut ctx, "#/token/unit", &json!("px"));
        let rem_state = DimensionUnit::try_from_json(&mut ctx, "#/token/unit", &json!("rem"));

        assert!(matches!(px_state, ParseState::Parsed(DimensionUnit::Px)));
        assert!(matches!(rem_state, ParseState::Parsed(DimensionUnit::Rem)));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn dimension_unit_returns_no_match_for_unsupported_values() {
        let mut ctx = test_ctx();

        let state = DimensionUnit::try_from_json(&mut ctx, "#/token/unit", &json!("em"));

        assert!(matches!(state, ParseState::Invalid));
        assert!(ctx.errors.len() == 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyValue);
        assert_eq!(ctx.errors[0].path, "#/token/unit");
        assert!(
            ctx.errors[0]
                .message
                .contains("Expected either 'px' or 'rem' for unit at #/token/unit")
        );
    }

    #[test]
    fn dimension_token_value_parses_literal_value_and_unit() {
        let mut ctx = test_ctx();
        let input = json!({ "value": 16, "unit": "px" });

        let state = DimensionTokenValue::try_from_json(&mut ctx, "#/token", &input);

        let parsed = match state {
            ParseState::Parsed(v) => v,
            _ => panic!("expected parsed dimension token value"),
        };

        assert_eq!(
            parsed.value,
            RefOrLiteral::Literal(DimensionValue(JsonNumber(serde_json::Number::from(16))))
        );
        assert_eq!(parsed.unit, RefOrLiteral::Literal(DimensionUnit::Px));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn dimension_token_value_parses_json_references_for_value_and_unit() {
        let mut ctx = test_ctx();
        let input = json!({
            "value": { "$ref": "#/scales/spacing/md" },
            "unit": { "$ref": "#/tokens/unit" }
        });

        let state = DimensionTokenValue::try_from_json(&mut ctx, "#/token", &input);

        let parsed = match state {
            ParseState::Parsed(v) => v,
            _ => panic!("expected parsed dimension token value"),
        };

        assert!(matches!(
            parsed.value,
            RefOrLiteral::Ref(JsonRefObject { .. })
        ));
        assert!(matches!(
            parsed.unit,
            RefOrLiteral::Ref(JsonRefObject { .. })
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn dimension_token_value_rejects_non_object_input() {
        let mut ctx = test_ctx();

        let state = DimensionTokenValue::try_from_json(&mut ctx, "#/token", &json!(42));

        assert!(matches!(state, ParseState::Invalid));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyType);
        assert_eq!(ctx.errors[0].path, "#/token");
    }

    #[test]
    fn dimension_token_value_reports_missing_required_fields() {
        let mut ctx = test_ctx();
        let input = json!({ "value": 16 });

        let state = DimensionTokenValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::Invalid));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[0].path, "#/token/unit");
    }

    #[test]
    fn dimension_token_value_reports_invalid_unit_value() {
        let mut ctx = test_ctx();
        let input = json!({ "value": 16, "unit": "em" });

        let state = DimensionTokenValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::Invalid));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyValue);
        assert_eq!(ctx.errors[0].path, "#/token/unit");
        assert!(
            ctx.errors[0]
                .message
                .contains("Expected either 'px' or 'rem'")
        );
    }
}
