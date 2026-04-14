//! The `font_weight` module defines the `FontWeightTokenValue` struct which represents the DTCG font-weight token type.

use crate::ir::{JsonNumber, ParseState, RefOrLiteral, TryFromJson};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FontWeightValueString {
    Thin,
    Hairline,
    ExtraLight,
    UltraLight,
    Light,
    Normal,
    Medium,
    SemiBold,
    Bold,
    ExtraBold,
    UltraBold,
    Black,
    Heavy,
    ExtraBlack,
    UltraBlack,
}

impl<'a> TryFromJson<'a> for FontWeightValueString {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match value {
            serde_json::Value::String(s) => match s.as_str() {
                "thin" => ParseState::Parsed(Self::Thin),
                "hairline" => ParseState::Parsed(Self::Hairline),
                "extra-light" => ParseState::Parsed(Self::ExtraLight),
                "ultra-light" => ParseState::Parsed(Self::UltraLight),
                "light" => ParseState::Parsed(Self::Light),
                "normal" => ParseState::Parsed(Self::Normal),
                "regular" => ParseState::Parsed(Self::Normal),
                "book" => ParseState::Parsed(Self::Normal),
                "medium" => ParseState::Parsed(Self::Medium),
                "semi-bold" => ParseState::Parsed(Self::SemiBold),
                "demi-bold" => ParseState::Parsed(Self::SemiBold),
                "bold" => ParseState::Parsed(Self::Bold),
                "extra-bold" => ParseState::Parsed(Self::ExtraBold),
                "ultra-bold" => ParseState::Parsed(Self::UltraBold),
                "black" => ParseState::Parsed(Self::Black),
                "heavy" => ParseState::Parsed(Self::Heavy),
                "extra-black" => ParseState::Parsed(Self::ExtraBlack),
                "ultra-black" => ParseState::Parsed(Self::UltraBlack),
                _ => {
                    ctx.push_to_errors(
                        crate::errors::DiagnosticCode::InvalidPropertyValue,
                        format!("Invalid font-weight string value '{}' at {}", s, path),
                        path.into(),
                    );
                    ParseState::Invalid
                }
            },
            _ => ParseState::NoMatch,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontWeightValueNumber(pub JsonNumber);

impl<'a> TryFromJson<'a> for FontWeightValueNumber {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match JsonNumber::try_from_json(ctx, path, value) {
            ParseState::Parsed(number) => ParseState::Parsed(Self(number)),
            _ => ParseState::Invalid,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FontWeightValue {
    String(FontWeightValueString),
    Number(FontWeightValueNumber),
}

impl<'a> TryFromJson<'a> for FontWeightValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match FontWeightValueString::try_from_json(ctx, path, value) {
            ParseState::Parsed(string_val) => ParseState::Parsed(Self::String(string_val)),
            ParseState::Invalid => ParseState::Invalid,
            ParseState::NoMatch => match FontWeightValueNumber::try_from_json(ctx, path, value) {
                ParseState::Parsed(number_val) => ParseState::Parsed(Self::Number(number_val)),
                _ => ParseState::Invalid,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FontWeightTokenValue(pub RefOrLiteral<FontWeightValue>);

impl<'a> TryFromJson<'a> for FontWeightTokenValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        let result = match RefOrLiteral::<FontWeightValue>::try_from_json(ctx, path, value) {
            ParseState::Parsed(res) => res,
            ParseState::Invalid => {
                ctx.push_to_errors(
                    crate::errors::DiagnosticCode::InvalidTokenValue,
                    format!("Expected a font-weight token value (either a string like 'bold' or a number like 700) at {}", path),
                    path.into(),
                );
                return ParseState::Invalid;
            }
            ParseState::NoMatch => return ParseState::NoMatch,
        };

        ParseState::Parsed(Self(result))
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
    fn font_weight_value_string_parses_canonical_values() {
        let mut ctx = test_ctx();

        let bold = FontWeightValueString::try_from_json(&mut ctx, "#/token", &json!("bold"));
        let thin = FontWeightValueString::try_from_json(&mut ctx, "#/token", &json!("thin"));

        assert!(matches!(bold, ParseState::Parsed(FontWeightValueString::Bold)));
        assert!(matches!(thin, ParseState::Parsed(FontWeightValueString::Thin)));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn font_weight_value_string_parses_alias_values_to_current_variants() {
        let mut ctx = test_ctx();

        let regular =
            FontWeightValueString::try_from_json(&mut ctx, "#/token", &json!("regular"));
        let book = FontWeightValueString::try_from_json(&mut ctx, "#/token", &json!("book"));
        let demi_bold =
            FontWeightValueString::try_from_json(&mut ctx, "#/token", &json!("demi-bold"));

        assert!(matches!(
            regular,
            ParseState::Parsed(FontWeightValueString::Normal)
        ));
        assert!(matches!(book, ParseState::Parsed(FontWeightValueString::Normal)));
        assert!(matches!(
            demi_bold,
            ParseState::Parsed(FontWeightValueString::SemiBold)
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn font_weight_value_string_rejects_invalid_string() {
        let mut ctx = test_ctx();

        let state =
            FontWeightValueString::try_from_json(&mut ctx, "#/token", &json!("super-bold"));

        assert!(matches!(state, ParseState::Invalid));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyValue);
        assert_eq!(ctx.errors[0].path, "#/token");
    }

    #[test]
    fn font_weight_value_string_returns_no_match_for_non_string() {
        let mut ctx = test_ctx();

        let state = FontWeightValueString::try_from_json(&mut ctx, "#/token", &json!(700));

        assert!(matches!(state, ParseState::NoMatch));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn font_weight_value_number_parses_number() {
        let mut ctx = test_ctx();

        let state = FontWeightValueNumber::try_from_json(&mut ctx, "#/token", &json!(700));

        assert!(matches!(
            state,
            ParseState::Parsed(FontWeightValueNumber(JsonNumber(_)))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn font_weight_value_parses_string_and_number_variants() {
        let mut ctx = test_ctx();

        let string_state = FontWeightValue::try_from_json(&mut ctx, "#/token", &json!("black"));
        let number_state = FontWeightValue::try_from_json(&mut ctx, "#/token", &json!(900));

        assert!(matches!(
            string_state,
            ParseState::Parsed(FontWeightValue::String(FontWeightValueString::Black))
        ));
        assert!(matches!(
            number_state,
            ParseState::Parsed(FontWeightValue::Number(FontWeightValueNumber(JsonNumber(_))))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn font_weight_token_value_parses_literal_string() {
        let mut ctx = test_ctx();

        let state = FontWeightTokenValue::try_from_json(&mut ctx, "#/token", &json!("bold"));

        assert!(matches!(
            state,
            ParseState::Parsed(FontWeightTokenValue(RefOrLiteral::Literal(
                FontWeightValue::String(FontWeightValueString::Bold)
            )))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn font_weight_token_value_parses_literal_number() {
        let mut ctx = test_ctx();

        let state = FontWeightTokenValue::try_from_json(&mut ctx, "#/token", &json!(600));

        assert!(matches!(
            state,
            ParseState::Parsed(FontWeightTokenValue(RefOrLiteral::Literal(
                FontWeightValue::Number(FontWeightValueNumber(JsonNumber(_)))
            )))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn font_weight_token_value_parses_reference() {
        let mut ctx = test_ctx();

        let state = FontWeightTokenValue::try_from_json(
            &mut ctx,
            "#/token",
            &json!({ "$ref": "#/typography/weight/body" }),
        );

        assert!(matches!(
            state,
            ParseState::Parsed(FontWeightTokenValue(RefOrLiteral::Ref(_)))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn font_weight_token_value_reports_invalid_string_and_invalid_token_value() {
        let mut ctx = test_ctx();

        let state =
            FontWeightTokenValue::try_from_json(&mut ctx, "#/token", &json!("super-bold"));

        assert!(matches!(state, ParseState::Invalid));
        assert!(ctx
            .errors
            .iter()
            .any(|e| e.code == DiagnosticCode::InvalidPropertyValue && e.path == "#/token"));
        assert!(ctx
            .errors
            .iter()
            .any(|e| e.code == DiagnosticCode::InvalidTokenValue && e.path == "#/token"));
    }

    #[test]
    fn font_weight_token_value_reports_invalid_ref_shape() {
        let mut ctx = test_ctx();

        let state = FontWeightTokenValue::try_from_json(
            &mut ctx,
            "#/token",
            &json!({ "$ref": "#/typography/weight/body", "extra": true }),
        );

        assert!(matches!(state, ParseState::Invalid));
        assert!(ctx
            .errors
            .iter()
            .any(|e| e.code == DiagnosticCode::InvalidReference && e.path == "#/token"));
        assert!(ctx
            .errors
            .iter()
            .any(|e| e.code == DiagnosticCode::InvalidTokenValue && e.path == "#/token"));
    }
}
