//! The `font_family` module defines the `FontFamilyTokenValue` struct which represents the DTCG font-family token type.

use crate::{
    errors::DiagnosticCode,
    ir::{JsonArray, ParseState, RefOrLiteral, TryFromJson},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontFamilyMultipleValue(pub Vec<RefOrLiteral<String>>);

impl<'a> TryFromJson<'a> for FontFamilyMultipleValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        let array = match value {
            serde_json::Value::Array(arr) => JsonArray(arr),
            _ => return ParseState::NoMatch,
        };

        let result = match array.parse_for_each::<RefOrLiteral<String>>(ctx, path) {
            Some(res) => res,
            None => {
                ctx.push_to_errors(
                    crate::errors::DiagnosticCode::InvalidPropertyValue,
                    format!(
                        "Expected all entries to be either strings or references to strings for font-family token at {}",
                        path
                    ),
                    path.into(),
                );
                return ParseState::Invalid;
            }
        };

        ParseState::Parsed(Self(result))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FontFamilyValue {
    Single(RefOrLiteral<String>),
    Multiple(RefOrLiteral<FontFamilyMultipleValue>),
}

impl<'a> TryFromJson<'a> for FontFamilyValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match RefOrLiteral::<String>::try_from_json(ctx, path, value) {
            ParseState::Parsed(single) => ParseState::Parsed(Self::Single(single)),
            ParseState::Invalid => ParseState::Invalid,
            ParseState::NoMatch => {
                match RefOrLiteral::<FontFamilyMultipleValue>::try_from_json(ctx, path, value) {
                    ParseState::Parsed(multiple) => ParseState::Parsed(Self::Multiple(multiple)),
                    ParseState::Invalid => ParseState::Invalid,
                    ParseState::NoMatch => ParseState::NoMatch,
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontFamilyTokenValue(pub FontFamilyValue);

impl<'a> TryFromJson<'a> for FontFamilyTokenValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match FontFamilyValue::try_from_json(ctx, path, value) {
            ParseState::Parsed(font_family) => ParseState::Parsed(Self(font_family)),
            _ => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidTokenValue, 
                    format!("Expected a string, JSON reference, or array of strings / JSON references, but got {:?}", value), 
                    path.into()
                );
                ParseState::Invalid
            }
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
        ir::{ParseState, TryFromJson},
    };

    fn test_ctx() -> ParserContext {
        ParserContext::new("test.json".to_string(), FileFormat::Json, "{}".to_string())
    }

    #[test]
    fn font_family_multiple_value_parses_string_array() {
        let mut ctx = test_ctx();
        let input = json!(["Inter", "sans-serif"]);

        let state = FontFamilyMultipleValue::try_from_json(&mut ctx, "#/token", &input);

        let parsed = match state {
            ParseState::Parsed(v) => v,
            _ => panic!("expected parsed multiple font family value"),
        };

        assert_eq!(parsed.0.len(), 2);
        assert!(matches!(parsed.0[0], RefOrLiteral::Literal(_)));
        assert!(matches!(parsed.0[1], RefOrLiteral::Literal(_)));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn font_family_multiple_value_parses_reference_array() {
        let mut ctx = test_ctx();
        let input = json!([
            { "$ref": "#/fonts/heading" },
            { "$ref": "#/fonts/fallback" }
        ]);

        let state = FontFamilyMultipleValue::try_from_json(&mut ctx, "#/token", &input);

        let parsed = match state {
            ParseState::Parsed(v) => v,
            _ => panic!("expected parsed multiple font family value"),
        };

        assert_eq!(parsed.0.len(), 2);
        assert!(matches!(parsed.0[0], RefOrLiteral::Ref(_)));
        assert!(matches!(parsed.0[1], RefOrLiteral::Ref(_)));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn font_family_multiple_value_returns_no_match_for_non_array() {
        let mut ctx = test_ctx();

        let state = FontFamilyMultipleValue::try_from_json(&mut ctx, "#/token", &json!("Inter"));

        assert!(matches!(state, ParseState::NoMatch));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn font_family_multiple_value_rejects_invalid_array_element() {
        let mut ctx = test_ctx();
        let input = json!(["Inter", 12, "sans-serif"]);

        let state = FontFamilyMultipleValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::Invalid));
        assert!(ctx.errors.iter().any(|e| {
            e.code == DiagnosticCode::InvalidPropertyType && e.path == "#/token/1"
        }));
        assert!(ctx.errors.iter().any(|e| {
            e.code == DiagnosticCode::InvalidPropertyValue && e.path == "#/token"
        }));
    }

    #[test]
    fn font_family_value_parses_single_string() {
        let mut ctx = test_ctx();

        let state = FontFamilyValue::try_from_json(&mut ctx, "#/token", &json!("Inter"));

        assert!(matches!(
            state,
            ParseState::Parsed(FontFamilyValue::Single(RefOrLiteral::Literal(_)))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn font_family_value_prefers_single_variant_for_top_level_reference() {
        let mut ctx = test_ctx();

        let state = FontFamilyValue::try_from_json(
            &mut ctx,
            "#/token",
            &json!({ "$ref": "#/fonts/body" }),
        );

        assert!(matches!(
            state,
            ParseState::Parsed(FontFamilyValue::Single(RefOrLiteral::Ref(_)))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn font_family_value_parses_multiple_literal_array() {
        let mut ctx = test_ctx();

        let state = FontFamilyValue::try_from_json(
            &mut ctx,
            "#/token",
            &json!(["Inter", "Arial", "sans-serif"]),
        );

        assert!(matches!(
            state,
            ParseState::Parsed(FontFamilyValue::Multiple(RefOrLiteral::Literal(_)))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn font_family_token_value_parses_multiple_literal_array() {
        let mut ctx = test_ctx();
        let input = json!(["Inter", "Arial", "sans-serif"]);

        let state = FontFamilyTokenValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(
            state,
            ParseState::Parsed(FontFamilyTokenValue(FontFamilyValue::Multiple(
                RefOrLiteral::Literal(_)
            )))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn font_family_token_value_rejects_invalid_scalar() {
        let mut ctx = test_ctx();

        let state = FontFamilyTokenValue::try_from_json(&mut ctx, "#/token", &json!(123));

        assert!(matches!(state, ParseState::Invalid));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(ctx.errors[0].path, "#/token");
    }

    #[test]
    fn font_family_token_value_rejects_invalid_array_entry() {
        let mut ctx = test_ctx();
        let input = json!(["Inter", false]);

        let state = FontFamilyTokenValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::Invalid));
        assert!(ctx.errors.iter().any(|e| {
            e.code == DiagnosticCode::InvalidPropertyType && e.path == "#/token/1"
        }));
        assert!(ctx.errors.iter().any(|e| {
            e.code == DiagnosticCode::InvalidPropertyValue && e.path == "#/token"
        }));
        assert!(ctx.errors.iter().any(|e| {
            e.code == DiagnosticCode::InvalidTokenValue && e.path == "#/token"
        }));
    }

    #[test]
    fn font_family_token_value_reports_invalid_ref_shape() {
        let mut ctx = test_ctx();
        let input = json!({ "$ref": "#/fonts/body", "extra": true });

        let state = FontFamilyTokenValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::Invalid));
        assert!(ctx.errors.iter().any(|e| {
            e.code == DiagnosticCode::InvalidReference && e.path == "#/token"
        }));
        assert!(ctx.errors.iter().any(|e| {
            e.code == DiagnosticCode::InvalidTokenValue && e.path == "#/token"
        }));
    }
}
