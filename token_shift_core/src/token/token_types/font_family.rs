//! The `font_family` module defines the data structures for font family tokens defined in the DTCG specification,
//! which represents a font family in the UI, such as the name of the font and its fallback fonts.

use crate::token::{ParseState, TryFromJson};

pub enum FontFamilyValue {
    Single(String),
    Multiple(Vec<String>),
}

pub struct FontFamilyTokenValue {
    pub value: FontFamilyValue,
}

impl<'a> TryFromJson<'a> for FontFamilyTokenValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match value {
            serde_json::Value::String(str_value) => ParseState::Parsed(FontFamilyTokenValue {
                value: FontFamilyValue::Single(str_value.clone()),
            }),
            serde_json::Value::Array(arr_value) => {
                let mut font_families = Vec::new();
                for (index, item) in arr_value.iter().enumerate() {
                    match item {
                        serde_json::Value::String(str_value) => {
                            font_families.push(str_value.clone())
                        }
                        _ => {
                            ctx.push_to_errors(
                                crate::errors::DiagnosticCode::InvalidTokenValue,
                                format!(
                                    "expected array items to be strings for font family token, but found {}",
                                    item
                                ),
                                format!("{}.{}", path, index),
                            );
                            return ParseState::Skipped;
                        }
                    }
                }
                ParseState::Parsed(FontFamilyTokenValue {
                    value: FontFamilyValue::Multiple(font_families),
                })
            }
            _ => {
                ctx.push_to_errors(
                    crate::errors::DiagnosticCode::InvalidTokenValue,
                    format!(
                        "expected font family token value to be either a string or an array of strings, but found {}",
                        value
                    ),
                    path.into(),
                );
                ParseState::Skipped
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FileFormat, ParserContext, errors::DiagnosticCode};
    use serde_json::json;

    fn make_context() -> ParserContext {
        ParserContext::new(String::from("test.json"), FileFormat::Json, String::new())
    }

    fn parse_font_family(
        value: &serde_json::Value,
    ) -> (ParseState<FontFamilyTokenValue>, ParserContext) {
        let mut ctx = make_context();
        let result = FontFamilyTokenValue::try_from_json(
            &mut ctx,
            "tokens.typography.body.fontFamily",
            value,
        );
        (result, ctx)
    }

    #[test]
    fn parses_single_font_family_string() {
        let value = json!("Inter");

        let (result, ctx) = parse_font_family(&value);

        assert!(ctx.errors.is_empty());

        let ParseState::Parsed(parsed) = result else {
            panic!("expected a parsed font family token");
        };

        match parsed.value {
            FontFamilyValue::Single(name) => assert_eq!(name, "Inter"),
            FontFamilyValue::Multiple(_) => panic!("expected a single font family value"),
        }
    }

    #[test]
    fn parses_multiple_font_families_array() {
        let value = json!(["Inter", "Helvetica", "sans-serif"]);

        let (result, ctx) = parse_font_family(&value);

        assert!(ctx.errors.is_empty());

        let ParseState::Parsed(parsed) = result else {
            panic!("expected a parsed font family token");
        };

        match parsed.value {
            FontFamilyValue::Multiple(families) => {
                assert_eq!(families, vec!["Inter", "Helvetica", "sans-serif"]);
            }
            FontFamilyValue::Single(_) => panic!("expected multiple font family values"),
        }
    }

    #[test]
    fn parses_empty_font_family_array() {
        let value = json!([]);

        let (result, ctx) = parse_font_family(&value);

        assert!(ctx.errors.is_empty());

        let ParseState::Parsed(parsed) = result else {
            panic!("expected a parsed font family token");
        };

        match parsed.value {
            FontFamilyValue::Multiple(families) => assert!(families.is_empty()),
            FontFamilyValue::Single(_) => panic!("expected multiple font family values"),
        }
    }

    #[test]
    fn skips_when_array_contains_non_string_item() {
        let value = json!(["Inter", 42, "sans-serif"]);

        let (result, ctx) = parse_font_family(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(
            ctx.errors[0].message,
            "expected array items to be strings for font family token, but found 42"
        );
        assert_eq!(ctx.errors[0].path, "tokens.typography.body.fontFamily.1");
    }

    #[test]
    fn stops_at_first_invalid_array_item() {
        let value = json!([true, 42]);

        let (result, ctx) = parse_font_family(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(
            ctx.errors[0].message,
            "expected array items to be strings for font family token, but found true"
        );
        assert_eq!(ctx.errors[0].path, "tokens.typography.body.fontFamily.0");
    }

    #[test]
    fn skips_for_invalid_non_string_non_array_values() {
        let cases = [
            json!(42),
            json!(true),
            json!(null),
            json!({"primary": "Inter"}),
        ];

        for value in cases {
            let (result, ctx) = parse_font_family(&value);

            assert!(matches!(result, ParseState::Skipped));
            assert_eq!(ctx.errors.len(), 1);
            assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
            assert_eq!(ctx.errors[0].path, "tokens.typography.body.fontFamily");
            assert_eq!(
                ctx.errors[0].message,
                format!(
                    "expected font family token value to be either a string or an array of strings, but found {}",
                    value
                )
            );
        }
    }
}
