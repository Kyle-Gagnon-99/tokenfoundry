//! The `font_weight` module defines the data structures for font weight tokens defined in the DTCG specification,
//! which represents the weight of a font in the UI, such as normal, bold, or numeric values like 400, 700, etc.

use crate::ir::{JsonNumber, RefOr, TryFromJson, parse_ref_or_value};

/// The `FontWeightValueString` enum represents the allowed string values for font weight tokens, as defined in the DTCG specification.
/// This uses the OpenType wght tag specifcation for font weight strings
#[derive(Debug, Clone, PartialEq)]
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
        match s {
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
#[derive(Debug, Clone, PartialEq)]
pub enum FontWeightValue {
    Numeric(JsonNumber),
    String(FontWeightValueString),
}

impl<'a> TryFromJson<'a> for FontWeightValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        match value {
            serde_json::Value::Number(_) => Some(FontWeightValue::Numeric(
                JsonNumber::from_value(value).unwrap(),
            )),
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
#[derive(Debug, Clone, PartialEq)]
pub struct FontWeightTokenValue(RefOr<FontWeightValue>);

impl<'a> TryFromJson<'a> for FontWeightTokenValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        parse_ref_or_value(ctx, path, value).map(FontWeightTokenValue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        FileFormat, ParserContext,
        errors::DiagnosticCode,
        ir::{JsonPointer, JsonRef, RefOr},
    };
    use serde_json::{Number, json};

    fn parser_context() -> ParserContext {
        ParserContext::new("tests.json".into(), FileFormat::Json, String::new())
    }

    #[test]
    fn parses_numeric_font_weight_value() {
        let mut ctx = parser_context();

        let parsed = FontWeightValue::try_from_json(&mut ctx, "/token", &json!(400));

        match parsed {
            Some(FontWeightValue::Numeric(JsonNumber(number))) => {
                assert_eq!(number, Number::from(400));
            }
            _ => panic!("expected numeric font weight"),
        }
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn parses_fractional_numeric_font_weight_value() {
        let mut ctx = parser_context();

        let parsed = FontWeightValue::try_from_json(&mut ctx, "/token", &json!(425.5));

        match parsed {
            Some(FontWeightValue::Numeric(JsonNumber(number))) => {
                assert_eq!(number, Number::from_f64(425.5).unwrap());
            }
            _ => panic!("expected numeric font weight"),
        }
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn parses_canonical_font_weight_strings() {
        let mut ctx = parser_context();

        let thin = FontWeightValue::try_from_json(&mut ctx, "/token", &json!("thin"));
        let bold = FontWeightValue::try_from_json(&mut ctx, "/token", &json!("bold"));
        let extra_black = FontWeightValue::try_from_json(&mut ctx, "/token", &json!("extra-black"));

        assert!(matches!(
            thin,
            Some(FontWeightValue::String(FontWeightValueString::Thin))
        ));
        assert!(matches!(
            bold,
            Some(FontWeightValue::String(FontWeightValueString::Bold))
        ));
        assert!(matches!(
            extra_black,
            Some(FontWeightValue::String(FontWeightValueString::ExtraBlack))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn rejects_unknown_font_weight_string() {
        let mut ctx = parser_context();

        let parsed = FontWeightValue::try_from_json(&mut ctx, "/token", &json!("superbold"));

        assert!(parsed.is_none());
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(ctx.errors[0].path, "/token");
        assert!(
            ctx.errors[0]
                .message
                .contains("Invalid font weight token string value")
        );
    }

    #[test]
    fn rejects_invalid_font_weight_value_type() {
        let mut ctx = parser_context();

        let parsed = FontWeightValue::try_from_json(&mut ctx, "/token", &json!([400]));

        assert!(parsed.is_none());
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(ctx.errors[0].path, "/token");
    }

    #[test]
    fn parses_font_weight_token_value_as_literal_number() {
        let mut ctx = parser_context();

        let parsed = FontWeightTokenValue::try_from_json(&mut ctx, "/token", &json!(700));

        match parsed {
            Some(FontWeightTokenValue(RefOr::Literal(FontWeightValue::Numeric(JsonNumber(
                number,
            ))))) => {
                assert_eq!(number, Number::from(700));
            }
            _ => panic!("expected literal numeric token value"),
        }
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn parses_font_weight_token_value_as_literal_string() {
        let mut ctx = parser_context();

        let parsed = FontWeightTokenValue::try_from_json(&mut ctx, "/token", &json!("book"));

        assert!(matches!(
            parsed,
            Some(FontWeightTokenValue(RefOr::Literal(
                FontWeightValue::String(FontWeightValueString::Normal,)
            )))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn parses_font_weight_token_value_as_top_level_reference() {
        let mut ctx = parser_context();

        let parsed = FontWeightTokenValue::try_from_json(
            &mut ctx,
            "/token",
            &json!({ "$ref": "#/fonts/body/weight" }),
        );

        match parsed {
            Some(FontWeightTokenValue(RefOr::Ref(json_ref))) => {
                assert_eq!(
                    json_ref,
                    JsonRef::new_local_pointer(
                        "#/fonts/body/weight".to_string(),
                        JsonPointer::from("#/fonts/body/weight")
                    )
                );
            }
            _ => panic!("expected top-level reference token value"),
        }
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn rejects_font_weight_token_value_with_invalid_top_level_reference() {
        let mut ctx = parser_context();

        let parsed = FontWeightTokenValue::try_from_json(
            &mut ctx,
            "/token",
            &json!({ "$ref": "not-a-pointer" }),
        );

        assert!(parsed.is_none());
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidReference);
        assert_eq!(ctx.errors[0].path, "/token");
    }

    #[test]
    fn rejects_font_weight_token_value_with_invalid_object_shape() {
        let mut ctx = parser_context();

        let parsed =
            FontWeightTokenValue::try_from_json(&mut ctx, "/token", &json!({ "weight": 400 }));

        assert!(parsed.is_none());
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(ctx.errors[0].path, "/token");
    }
}
