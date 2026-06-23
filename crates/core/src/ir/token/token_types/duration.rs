//! The `duration` module defines the `DurationTokenValue` struct which represents the DTCG duration token type.

use crate::{
    errors::DiagnosticCode,
    ir::{InvalidReason, JsonNumber, JsonObject, ParseState, RefOrLiteral, TryFromJson},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DurationValue(pub JsonNumber);

impl<'a> TryFromJson<'a> for DurationValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &JsonPointer,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match JsonNumber::try_from_json(ctx, path, value) {
            ParseState::Parsed(number) => ParseState::Parsed(Self(number)),
            _ => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidPropertyValue,
                    format!(
                        "Invalid duration value. Expected a number but got {:?}",
                        value
                    ),
                    path.into(),
                );
                ParseState::invalid_emitted(InvalidReason::InvalidFieldType)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DurationUnit {
    Ms,
    S,
}

impl<'a> TryFromJson<'a> for DurationUnit {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &JsonPointer,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match value {
            serde_json::Value::String(s) => match s.as_str() {
                "ms" => ParseState::Parsed(DurationUnit::Ms),
                "s" => ParseState::Parsed(DurationUnit::S),
                _ => {
                    ctx.push_to_errors(
                        DiagnosticCode::InvalidPropertyValue,
                        format!("Expected either 'ms' or 's' for duration unit at {}", path),
                        path.into(),
                    );
                    ParseState::invalid_emitted(InvalidReason::InvalidValue)
                }
            },
            _ => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidPropertyValue,
                    format!("Expected a string for duration unit at {}", path),
                    path.into(),
                );
                ParseState::invalid_emitted(InvalidReason::InvalidFieldType)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DurationTokenValue {
    pub value: RefOrLiteral<DurationValue>,
    pub unit: RefOrLiteral<DurationUnit>,
}

impl<'a> TryFromJson<'a> for DurationTokenValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &JsonPointer,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        let obj = match value {
            serde_json::Value::Object(map) => JsonObject::new(map),
            _ => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidPropertyValue,
                    format!("Expected an object for duration token value at {}", path),
                    path.into(),
                );
                return ParseState::invalid_emitted(InvalidReason::InvalidFieldType);
            }
        };

        let value = obj.required_field::<RefOrLiteral<DurationValue>>(ctx, path, "value");
        let unit = obj.required_field::<RefOrLiteral<DurationUnit>>(ctx, path, "unit");

        match (value, unit) {
            (Some(value), Some(unit)) => ParseState::Parsed(Self { value, unit }),
            _ => ParseState::invalid_emitted(InvalidReason::InvalidFieldType),
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
        ir::{JsonRefObject, ParseState, TryFromJson},
    };

    fn test_ctx() -> ParserContext {
        ParserContext::new("{}".to_string())
    }

    #[test]
    fn duration_value_parses_number_literal() {
        let mut ctx = test_ctx();

        let state = DurationValue::try_from_json(&mut ctx, "#/token/value", &json!(250));

        assert!(matches!(
            state,
            ParseState::Parsed(DurationValue(JsonNumber(_)))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn duration_value_rejects_non_number() {
        let mut ctx = test_ctx();

        let state = DurationValue::try_from_json(&mut ctx, "#/token/value", &json!("250"));

        assert!(matches!(state, ParseState::Failed(_)));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyValue);
        assert_eq!(ctx.errors[0].path, "#/token/value");
        assert!(
            ctx.errors[0]
                .message
                .contains("Invalid duration value. Expected a number but got")
        );
    }

    #[test]
    fn duration_unit_parses_supported_values() {
        let mut ctx = test_ctx();

        let cases = [
            (json!("ms"), DurationUnit::Ms),
            (json!("s"), DurationUnit::S),
        ];

        for (input, expected) in cases {
            let state = DurationUnit::try_from_json(&mut ctx, "#/token/unit", &input);
            assert!(matches!(state, ParseState::Parsed(unit) if unit == expected));
        }

        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn duration_unit_rejects_unsupported_string() {
        let mut ctx = test_ctx();

        let state = DurationUnit::try_from_json(&mut ctx, "#/token/unit", &json!("min"));

        assert!(matches!(state, ParseState::Failed(_)));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyValue);
        assert_eq!(ctx.errors[0].path, "#/token/unit");
        assert!(
            ctx.errors[0]
                .message
                .contains("Expected either 'ms' or 's'")
        );
    }

    #[test]
    fn duration_unit_rejects_non_string() {
        let mut ctx = test_ctx();

        let state = DurationUnit::try_from_json(&mut ctx, "#/token/unit", &json!(100));

        assert!(matches!(state, ParseState::Failed(_)));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyValue);
        assert_eq!(ctx.errors[0].path, "#/token/unit");
        assert!(
            ctx.errors[0]
                .message
                .contains("Expected a string for duration unit")
        );
    }

    #[test]
    fn duration_token_value_parses_literal_value_and_unit() {
        let mut ctx = test_ctx();
        let input = json!({ "value": 300, "unit": "ms" });

        let state = DurationTokenValue::try_from_json(&mut ctx, "#/token", &input);

        let parsed = match state {
            ParseState::Parsed(v) => v,
            _ => panic!("expected parsed duration token value"),
        };

        assert_eq!(
            parsed.value,
            RefOrLiteral::Literal(DurationValue(JsonNumber(serde_json::Number::from(300))))
        );
        assert_eq!(parsed.unit, RefOrLiteral::Literal(DurationUnit::Ms));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn duration_token_value_parses_refs_for_value_and_unit() {
        let mut ctx = test_ctx();
        let input = json!({
            "value": { "$ref": "#/motion/fast/value" },
            "unit": { "$ref": "#/motion/fast/unit" }
        });

        let state = DurationTokenValue::try_from_json(&mut ctx, "#/token", &input);

        let parsed = match state {
            ParseState::Parsed(v) => v,
            _ => panic!("expected parsed duration token value"),
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
    fn duration_token_value_rejects_non_object() {
        let mut ctx = test_ctx();

        let state = DurationTokenValue::try_from_json(&mut ctx, "#/token", &json!(1));

        assert!(matches!(state, ParseState::Failed(_)));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyValue);
        assert_eq!(ctx.errors[0].path, "#/token");
        assert!(
            ctx.errors[0]
                .message
                .contains("Expected an object for duration token value at #/token")
        );
    }

    #[test]
    fn duration_token_value_reports_missing_value_field() {
        let mut ctx = test_ctx();
        let input = json!({ "unit": "ms" });

        let state = DurationTokenValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::Failed(_)));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[0].path, "#/token/value");
    }

    #[test]
    fn duration_token_value_reports_missing_unit_field() {
        let mut ctx = test_ctx();
        let input = json!({ "value": 150 });

        let state = DurationTokenValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::Failed(_)));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[0].path, "#/token/unit");
    }

    #[test]
    fn duration_token_value_reports_invalid_value_type() {
        let mut ctx = test_ctx();
        let input = json!({ "value": "300", "unit": "ms" });

        let state = DurationTokenValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::Failed(_)));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyValue);
        assert_eq!(ctx.errors[0].path, "#/token/value");
    }

    #[test]
    fn duration_token_value_reports_invalid_unit_value() {
        let mut ctx = test_ctx();
        let input = json!({ "value": 300, "unit": "min" });

        let state = DurationTokenValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::Failed(_)));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyValue);
        assert_eq!(ctx.errors[0].path, "#/token/unit");
    }

    #[test]
    fn duration_token_value_collects_errors_for_both_invalid_fields() {
        let mut ctx = test_ctx();
        let input = json!({ "value": "bad", "unit": 100 });

        let state = DurationTokenValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::Failed(_)));
        assert_eq!(ctx.errors.len(), 2);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyValue);
        assert_eq!(ctx.errors[0].path, "#/token/value");
        assert_eq!(ctx.errors[1].code, DiagnosticCode::InvalidPropertyValue);
        assert_eq!(ctx.errors[1].path, "#/token/unit");
    }

    #[test]
    fn duration_token_value_reports_invalid_ref_object_for_value() {
        let mut ctx = test_ctx();
        let input = json!({
            "value": { "$ref": "#/motion/fast/value", "extra": true },
            "unit": "ms"
        });

        let state = DurationTokenValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::Failed(_)));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidReference);
        assert_eq!(ctx.errors[0].path, "#/token/value");
    }

    #[test]
    fn duration_token_value_reports_invalid_ref_object_for_unit() {
        let mut ctx = test_ctx();
        let input = json!({
            "value": 250,
            "unit": { "$ref": "#/motion/fast/unit", "extra": true }
        });

        let state = DurationTokenValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::Failed(_)));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidReference);
        assert_eq!(ctx.errors[0].path, "#/token/unit");
    }
}
