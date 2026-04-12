//! The `number` module defines the data structures for number tokens defined in the DTCG specification
//! which represents a numeric value in the UI, such as an opacity value or a z-index value.

use crate::ir::{JsonNumber, TryFromJson};

#[derive(Debug, Clone, PartialEq)]
pub struct NumberTokenValue(pub JsonNumber);

impl<'a> TryFromJson<'a> for NumberTokenValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        JsonNumber::try_from_json(ctx, path, value).map(NumberTokenValue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FileFormat, ParserContext, errors::DiagnosticCode, ir::TryFromJson};
    use serde_json::{Number, json};

    fn parser_context() -> ParserContext {
        ParserContext::new("tests.json".into(), FileFormat::Json, String::new())
    }

    #[test]
    fn parses_integer_number_token_value() {
        let mut ctx = parser_context();

        let parsed = NumberTokenValue::try_from_json(&mut ctx, "/token", &json!(42));

        assert_eq!(parsed, Some(NumberTokenValue(JsonNumber(Number::from(42)))));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn parses_fractional_number_token_value() {
        let mut ctx = parser_context();

        let parsed = NumberTokenValue::try_from_json(&mut ctx, "/token", &json!(-0.75));

        assert_eq!(
            parsed,
            Some(NumberTokenValue(JsonNumber(
                Number::from_f64(-0.75).unwrap()
            )))
        );
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn rejects_non_number_token_value() {
        let mut ctx = parser_context();

        let parsed = NumberTokenValue::try_from_json(&mut ctx, "/token", &json!("42"));

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyType);
        assert_eq!(ctx.errors[0].path, "/token");
    }
}
