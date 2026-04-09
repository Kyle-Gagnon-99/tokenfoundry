//! The `font_weight` module defines the data structures for font weight tokens defined in the DTCG specification,
//! which represents the weight of a font in the UI, such as normal, bold, or numeric values like 400, 700, etc.

use crate::{
    ir::{RefOr, parse_ref_or_value},
    token::{TryFromJson, TryFromJsonField, utils::FloatOrInteger},
};

/// The `FontWeightValueString` enum represents the allowed string values for font weight tokens, as defined in the DTCG specification.
/// This uses the OpenType wght tag specifcation for font weight strings
pub enum FontWeightValueString {
    Thin,
    Hairline,
    ExtraLight,
    UltraLight,
    Light,
    Normal,
    Regular,
    Book,
    Medium,
    SemiBold,
    DemiBold,
    Bold,
    ExtraBold,
    UltraBold,
    Black,
    Heavy,
    ExtraBlack,
    UltraBlack,
}

/// The `FromStr` implementation for `FontWeightValueString` allows parsing a string into a `FontWeightValueString` enum variant, while ignoring case and allowing for common synonyms as defined in the OpenType specification for font weight strings.
/// If the input string does not match any of the allowed font weight strings, it returns an error (in this case, an empty tuple `()`, but more details will be provided later).
impl std::str::FromStr for FontWeightValueString {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "thin" => Ok(FontWeightValueString::Thin),
            "hairline" => Ok(FontWeightValueString::Hairline),
            "extra-light" | "ultra-light" => Ok(FontWeightValueString::ExtraLight),
            "light" => Ok(FontWeightValueString::Light),
            "normal" | "regular" | "book" => Ok(FontWeightValueString::Normal),
            "medium" => Ok(FontWeightValueString::Medium),
            "semi-bold" | "demi-bold" => Ok(FontWeightValueString::SemiBold),
            "bold" => Ok(FontWeightValueString::Bold),
            "extra-bold" | "ultra-bold" => Ok(FontWeightValueString::ExtraBold),
            "black" | "heavy" => Ok(FontWeightValueString::Black),
            "extra-black" | "ultra-black" => Ok(FontWeightValueString::ExtraBlack),
            _ => Err(()),
        }
    }
}

/// The `FontWeightValue` enum represents the value of a font weight token, which can be either a numeric value (e.g. 400, 700) or a string value (e.g. "normal", "bold")
pub enum FontWeightValue {
    Numeric(FloatOrInteger),
    String(FontWeightValueString),
}

impl<'a> TryFromJsonField<'a> for FontWeightValue {
    fn try_from_json_field(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        match value {
            serde_json::Value::Number(number) => match number.try_into() {
                Ok(parsed_number) => Some(FontWeightValue::Numeric(parsed_number)),
                Err(_) => {
                    ctx.push_to_errors(
                        crate::errors::DiagnosticCode::InvalidTokenValue,
                        format!(
                            "Invalid font weight token numeric value '{}', expected a valid number",
                            number
                        ),
                        path.into(),
                    );
                    None
                }
            },
            serde_json::Value::String(string_val) => {
                match string_val.parse::<FontWeightValueString>() {
                    Ok(parsed_string) => Some(FontWeightValue::String(parsed_string)),
                    Err(_) => {
                        ctx.push_to_errors(
                            crate::errors::DiagnosticCode::InvalidTokenValue,
                            format!(
                                "Invalid font weight token string value '{}', expected one of the following: thin, hairline, extra-light, ultra-light, light, normal, regular, book, medium, semi-bold, demi-bold, bold, extra-bold, ultra-bold, black, heavy, extra-black, ultra-black",
                                string_val
                            ),
                            path.into(),
                        );
                        None
                    }
                }
            }
            _ => {
                ctx.push_to_errors(
                    crate::errors::DiagnosticCode::InvalidTokenValue,
                    format!(
                        "Expected font weight token value to be either a number or a string, but found {}",
                        value
                    ),
                    path.into(),
                );
                None
            }
        }
    }
}

#[allow(dead_code)]
pub struct FontWeightTokenValue(RefOr<FontWeightValue>);

impl<'a> TryFromJson<'a> for FontWeightTokenValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> crate::token::ParseState<Self> {
        match parse_ref_or_value::<FontWeightValue>(ctx, path, value) {
            Some(v) => crate::token::ParseState::Parsed(FontWeightTokenValue(v)),
            None => crate::token::ParseState::Skipped,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        FileFormat, ParserContext,
        errors::DiagnosticCode,
        ir::{JsonPointer, JsonRef},
        token::ParseState,
    };
    use serde_json::json;

    fn make_context() -> ParserContext {
        ParserContext::new(String::from("test.json"), FileFormat::Json, String::new())
    }

    fn parse_font_weight(
        value: &serde_json::Value,
    ) -> (ParseState<FontWeightTokenValue>, ParserContext) {
        let mut ctx = make_context();
        let result = FontWeightTokenValue::try_from_json(
            &mut ctx,
            "tokens.typography.body.fontWeight",
            value,
        );
        (result, ctx)
    }

    #[test]
    fn parses_numeric_font_weight() {
        let value = json!(400);

        let (result, ctx) = parse_font_weight(&value);

        assert!(ctx.errors.is_empty());

        let ParseState::Parsed(FontWeightTokenValue(parsed)) = result else {
            panic!("expected parsed font weight token");
        };

        assert!(matches!(
            parsed,
            RefOr::Literal(FontWeightValue::Numeric(FloatOrInteger::Integer(400)))
        ));
    }

    #[test]
    fn parses_string_font_weight_case_insensitively() {
        let value = json!("BoLd");

        let (result, ctx) = parse_font_weight(&value);

        assert!(ctx.errors.is_empty());

        let ParseState::Parsed(FontWeightTokenValue(parsed)) = result else {
            panic!("expected parsed font weight token");
        };

        assert!(matches!(
            parsed,
            RefOr::Literal(FontWeightValue::String(FontWeightValueString::Bold))
        ));
    }

    #[test]
    fn parses_string_synonym_as_canonical_variant() {
        let value = json!("regular");

        let (result, ctx) = parse_font_weight(&value);

        assert!(ctx.errors.is_empty());

        let ParseState::Parsed(FontWeightTokenValue(parsed)) = result else {
            panic!("expected parsed font weight token");
        };

        assert!(matches!(
            parsed,
            RefOr::Literal(FontWeightValue::String(FontWeightValueString::Normal))
        ));
    }

    #[test]
    fn parses_empty_string_ref() {
        let value = json!({ "$ref": "" });

        let (result, ctx) = parse_font_weight(&value);

        assert!(ctx.errors.is_empty());

        let ParseState::Parsed(FontWeightTokenValue(parsed)) = result else {
            panic!("expected parsed font weight token");
        };

        assert!(matches!(
            parsed,
            RefOr::Ref(JsonRef {
                raw_value,
                kind: crate::ir::JsonRefKind::LocalPointer { pointer }
            }) if raw_value.is_empty() && pointer == JsonPointer::new()
        ));
    }

    #[test]
    fn parses_local_json_pointer_ref() {
        let value = json!({ "$ref": "#/tokens/typography/body/fontWeight" });

        let (result, ctx) = parse_font_weight(&value);

        assert!(ctx.errors.is_empty());

        let ParseState::Parsed(FontWeightTokenValue(parsed)) = result else {
            panic!("expected parsed font weight token");
        };

        assert!(matches!(
            parsed,
            RefOr::Ref(JsonRef {
                raw_value,
                kind: crate::ir::JsonRefKind::LocalPointer { pointer }
            }) if raw_value == "#/tokens/typography/body/fontWeight"
                && pointer == JsonPointer::from("#/tokens/typography/body/fontWeight")
        ));
    }

    #[test]
    fn skips_invalid_font_weight_string() {
        let value = json!("super-bold");

        let (result, ctx) = parse_font_weight(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(
            ctx.errors[0].message,
            "Invalid font weight token string value 'super-bold', expected one of the following: thin, hairline, extra-light, ultra-light, light, normal, regular, book, medium, semi-bold, demi-bold, bold, extra-bold, ultra-bold, black, heavy, extra-black, ultra-black"
        );
        assert_eq!(ctx.errors[0].path, "tokens.typography.body.fontWeight");
    }

    #[test]
    fn skips_invalid_value_type() {
        let value = json!(true);

        let (result, ctx) = parse_font_weight(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(
            ctx.errors[0].message,
            "Expected font weight token value to be either a number or a string, but found true"
        );
        assert_eq!(ctx.errors[0].path, "tokens.typography.body.fontWeight");
    }

    #[test]
    fn skips_invalid_ref_pointer() {
        let value = json!({ "$ref": "tokens/typography/body/fontWeight" });

        let (result, ctx) = parse_font_weight(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidReference);
        assert_eq!(
            ctx.errors[0].message,
            "Invalid JSON pointer: tokens/typography/body/fontWeight"
        );
        assert_eq!(ctx.errors[0].path, "tokens.typography.body.fontWeight");
    }
}
