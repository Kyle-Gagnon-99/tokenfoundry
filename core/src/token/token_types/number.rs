//! The `number` module defines the `NumberTokenValue` struct which represents the DTCG number token type.

use crate::ir::{JsonNumber, ParseState, RefOrLiteral, TryFromJson};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NumberTokenValue(pub RefOrLiteral<JsonNumber>);

impl<'a> TryFromJson<'a> for NumberTokenValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match RefOrLiteral::<JsonNumber>::try_from_json(ctx, path, value) {
            ParseState::Parsed(number) => ParseState::Parsed(Self(number)),
            ParseState::Invalid => ParseState::Invalid,
            ParseState::NoMatch => ParseState::NoMatch,
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
        ir::{ParseState, RefOrLiteral, TryFromJson},
    };

    fn test_ctx() -> ParserContext {
        ParserContext::new("test.json".to_string(), FileFormat::Json, "{}".to_string())
    }

    #[test]
    fn number_token_value_parses_number_literal() {
        let mut ctx = test_ctx();

        let state = NumberTokenValue::try_from_json(&mut ctx, "#/token", &json!(42));

        let parsed = match state {
            ParseState::Parsed(v) => v,
            _ => panic!("expected parsed number token value"),
        };

        assert_eq!(
            parsed,
            NumberTokenValue(RefOrLiteral::Literal(JsonNumber(serde_json::Number::from(
                42
            ))))
        );
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn number_token_value_parses_json_reference() {
        let mut ctx = test_ctx();
        let input = json!({ "$ref": "#/scale/base" });

        let state = NumberTokenValue::try_from_json(&mut ctx, "#/token", &input);

        let parsed = match state {
            ParseState::Parsed(v) => v,
            _ => panic!("expected parsed number token value"),
        };

        assert!(matches!(parsed.0, RefOrLiteral::Ref(_)));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn number_token_value_rejects_non_number_literals() {
        let invalid_inputs = [json!("42"), json!(true), json!([1, 2]), json!(null)];

        for input in invalid_inputs {
            let mut ctx = test_ctx();
            let state = NumberTokenValue::try_from_json(&mut ctx, "#/token", &input);
            assert!(matches!(state, ParseState::Invalid));
            assert!(ctx.errors.is_empty());
        }
    }

    #[test]
    fn number_token_value_rejects_object_without_ref() {
        let mut ctx = test_ctx();
        let input = json!({ "value": 42 });

        let state = NumberTokenValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::Invalid));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn number_token_value_reports_invalid_ref_shape() {
        let mut ctx = test_ctx();
        let input = json!({ "$ref": "#/scale/base", "extra": true });

        let state = NumberTokenValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::Invalid));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidReference);
        assert_eq!(ctx.errors[0].path, "#/token");
    }

    #[test]
    fn number_token_value_rejects_non_string_ref_value() {
        let mut ctx = test_ctx();
        let input = json!({ "$ref": 123 });

        let state = NumberTokenValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::Invalid));
        assert!(ctx.errors.is_empty());
    }
}
