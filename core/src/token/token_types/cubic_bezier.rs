//! The `cubic_bezier` module defines the `CubicBezierTokenValue` struct which represents the DTCG cubic-bezier token type.

use crate::ir::{JsonArray, JsonNumber, ParseState, RefOrLiteral, TryFromJson};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CubicBezierTokenValue(pub [RefOrLiteral<JsonNumber>; 4]);

impl<'a> TryFromJson<'a> for CubicBezierTokenValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        let array = match value {
            serde_json::Value::Array(arr) => JsonArray(arr),
            _ => return ParseState::NoMatch,
        };

        if array.len() != 4 {
            ctx.push_to_errors(
                crate::errors::DiagnosticCode::InvalidPropertyValue,
                format!(
                    "Expected an array of 4 numbers for cubic-bezier token at {}",
                    path
                ),
                path.into(),
            );
            return ParseState::Invalid;
        }

        let result = match array.parse_for_each::<RefOrLiteral<JsonNumber>>(ctx, path) {
            Some(res) => res,
            None => {
                ctx.push_to_errors(
                    crate::errors::DiagnosticCode::InvalidPropertyValue,
                    format!(
                        "Expected all entries to be either numbers or references to numbers for cubic-bezier token at {}",
                        path
                    ),
                    path.into(),
                );
                return ParseState::Invalid;
            }
        };
        if result.len() != 4 {
            ctx.push_to_errors(
                crate::errors::DiagnosticCode::InvalidPropertyValue,
                format!(
                    "Expected an array of 4 numbers for cubic-bezier token at {}",
                    path
                ),
                path.into(),
            );
            return ParseState::Invalid;
        }

        ParseState::Parsed(Self([
            result[0].clone(),
            result[1].clone(),
            result[2].clone(),
            result[3].clone(),
        ]))
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{
        FileFormat, ParserContext,
        errors::DiagnosticCode,
        ir::{ParseState, TryFromJson},
    };

    fn test_ctx() -> ParserContext {
        ParserContext::new("test.json".to_string(), FileFormat::Json, "{}".to_string())
    }

    #[test]
    fn cubic_bezier_parses_literal_numbers() {
        let mut ctx = test_ctx();
        let input = json!([0, 0.42, 0.58, 1]);

        let state = CubicBezierTokenValue::try_from_json(&mut ctx, "#/token", &input);

        let parsed = match state {
            ParseState::Parsed(v) => v,
            _ => panic!("expected parsed cubic bezier value"),
        };

        assert!(matches!(parsed.0[0], RefOrLiteral::Literal(_)));
        assert!(matches!(parsed.0[1], RefOrLiteral::Literal(_)));
        assert!(matches!(parsed.0[2], RefOrLiteral::Literal(_)));
        assert!(matches!(parsed.0[3], RefOrLiteral::Literal(_)));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn cubic_bezier_parses_reference_entries() {
        let mut ctx = test_ctx();
        let input = json!([
            { "$ref": "#/motion/easing/0" },
            { "$ref": "#/motion/easing/1" },
            { "$ref": "#/motion/easing/2" },
            { "$ref": "#/motion/easing/3" }
        ]);

        let state = CubicBezierTokenValue::try_from_json(&mut ctx, "#/token", &input);

        let parsed = match state {
            ParseState::Parsed(v) => v,
            _ => panic!("expected parsed cubic bezier value"),
        };

        assert!(matches!(parsed.0[0], RefOrLiteral::Ref(_)));
        assert!(matches!(parsed.0[1], RefOrLiteral::Ref(_)));
        assert!(matches!(parsed.0[2], RefOrLiteral::Ref(_)));
        assert!(matches!(parsed.0[3], RefOrLiteral::Ref(_)));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn cubic_bezier_returns_no_match_for_non_array() {
        let mut ctx = test_ctx();

        let state = CubicBezierTokenValue::try_from_json(&mut ctx, "#/token", &json!("ease"));

        assert!(matches!(state, ParseState::NoMatch));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn cubic_bezier_rejects_array_with_wrong_length() {
        let mut ctx = test_ctx();
        let input = json!([0, 0.5, 1]);

        let state = CubicBezierTokenValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::Invalid));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyValue);
        assert_eq!(ctx.errors[0].path, "#/token");
    }

    #[test]
    fn cubic_bezier_rejects_when_any_entry_is_invalid_literal() {
        let mut ctx = test_ctx();
        let input = json!([0, "bad", 0.58, 1]);

        let state = CubicBezierTokenValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::Invalid));
        assert!(
            ctx.errors
                .iter()
                .any(|e| e.code == DiagnosticCode::InvalidPropertyValue && e.path == "#/token")
        );
        assert!(
            ctx.errors.iter().any(|e| {
                e.code == DiagnosticCode::InvalidPropertyType && e.path == "#/token/1"
            })
        );
    }

    #[test]
    fn cubic_bezier_reports_invalid_reference_object_and_invalid_shape() {
        let mut ctx = test_ctx();
        let input = json!([
            0,
            { "$ref": "#/motion/easing/1", "extra": true },
            0.58,
            1
        ]);

        let state = CubicBezierTokenValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::Invalid));
        assert!(
            ctx.errors
                .iter()
                .any(|e| e.code == DiagnosticCode::InvalidReference && e.path == "#/token/1")
        );
        assert!(
            ctx.errors.iter().any(|e| {
                e.code == DiagnosticCode::InvalidPropertyType && e.path == "#/token/1"
            })
        );
        assert!(
            ctx.errors
                .iter()
                .any(|e| e.code == DiagnosticCode::InvalidPropertyValue && e.path == "#/token")
        );
    }
}
