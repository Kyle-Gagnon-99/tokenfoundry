//! The `font_family` module defines the data structures for font family tokens defined in the DTCG specification,
//! which represents a font family in the UI, such as the name of the font and its fallback fonts.

use crate::{
    ir::{RefOr, parse_ref_or_value},
    token::{ParseState, TryFromJson, TryFromJsonField},
};

#[derive(Debug, Clone, PartialEq)]
pub enum FontFamilyValue {
    Single(RefOr<String>),
    Multiple(Vec<RefOr<String>>),
}

impl<'a> TryFromJsonField<'a> for FontFamilyValue {
    fn try_from_json_field(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        match value {
            serde_json::Value::String(str_val) => {
                Some(FontFamilyValue::Single(RefOr::Literal(str_val.clone())))
            }
            serde_json::Value::Array(arr_val) => {
                let mut font_families = Vec::new();
                for (index, item) in arr_val.iter().enumerate() {
                    let val =
                        parse_ref_or_value::<String>(ctx, &format!("{}.{}", path, index), item);
                    if let Some(v) = val {
                        font_families.push(v);
                    } else {
                        // If any item in the array is invalid, we skip the entire token and return None
                        return None;
                    }
                }
                Some(FontFamilyValue::Multiple(font_families))
            }
            _ => {
                ctx.push_to_errors(
                    crate::errors::DiagnosticCode::InvalidTokenValue,
                    format!(
                        "Expected font family token value to be either a string or an array of strings, but found {}",
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
pub struct FontFamilyTokenValue(RefOr<FontFamilyValue>);

impl<'a> TryFromJson<'a> for FontFamilyTokenValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match parse_ref_or_value::<FontFamilyValue>(ctx, path, value).map(FontFamilyTokenValue) {
            Some(parsed) => ParseState::Parsed(parsed),
            None => ParseState::Skipped, // The error has already been pushed by parse_ref_or_value, so we just return Skipped here
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
    };
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

    fn expect_parsed(result: ParseState<FontFamilyTokenValue>) -> RefOr<FontFamilyValue> {
        let ParseState::Parsed(FontFamilyTokenValue(parsed)) = result else {
            panic!("expected a parsed font family token");
        };

        parsed
    }

    #[test]
    fn parses_single_font_family_string() {
        let value = json!("Inter");

        let (result, ctx) = parse_font_family(&value);

        assert!(ctx.errors.is_empty());

        match expect_parsed(result) {
            RefOr::Literal(FontFamilyValue::Single(name)) => {
                assert_eq!(name, RefOr::Literal("Inter".to_string()))
            }
            RefOr::Literal(FontFamilyValue::Multiple(_)) => {
                panic!("expected a single font family value")
            }
            RefOr::Ref(_) => panic!("expected a literal font family value"),
        }
    }

    #[test]
    fn parses_multiple_font_families_array() {
        let value = json!(["Inter", "Helvetica", "sans-serif"]);

        let (result, ctx) = parse_font_family(&value);

        assert!(ctx.errors.is_empty());

        match expect_parsed(result) {
            RefOr::Literal(FontFamilyValue::Multiple(families)) => {
                assert_eq!(
                    families,
                    vec![
                        RefOr::Literal("Inter".to_string()),
                        RefOr::Literal("Helvetica".to_string()),
                        RefOr::Literal("sans-serif".to_string())
                    ]
                );
            }
            RefOr::Literal(FontFamilyValue::Single(_)) => {
                panic!("expected multiple font family values")
            }
            RefOr::Ref(_) => panic!("expected a literal font family value"),
        }
    }

    #[test]
    fn parses_empty_font_family_array() {
        let value = json!([]);

        let (result, ctx) = parse_font_family(&value);

        assert!(ctx.errors.is_empty());

        match expect_parsed(result) {
            RefOr::Literal(FontFamilyValue::Multiple(families)) => assert!(families.is_empty()),
            RefOr::Literal(FontFamilyValue::Single(_)) => {
                panic!("expected multiple font family values")
            }
            RefOr::Ref(_) => panic!("expected a literal font family value"),
        }
    }

    #[test]
    fn parses_array_with_literal_and_ref_values() {
        let value = json!([
            "Inter",
            { "$ref": "#/tokens/typography/brand/fontFamily" },
            { "$ref": "" }
        ]);

        let (result, ctx) = parse_font_family(&value);

        assert!(ctx.errors.is_empty());

        match expect_parsed(result) {
            RefOr::Literal(FontFamilyValue::Multiple(families)) => {
                assert_eq!(families.len(), 3);
                assert_eq!(families[0], RefOr::Literal("Inter".to_string()));
                assert_eq!(
                    families[1],
                    RefOr::Ref(JsonRef::new_local_pointer(
                        "#/tokens/typography/brand/fontFamily".to_string(),
                        JsonPointer::from("#/tokens/typography/brand/fontFamily"),
                    ))
                );
                assert_eq!(
                    families[2],
                    RefOr::Ref(JsonRef::new_local_pointer(
                        String::new(),
                        JsonPointer::new(),
                    ))
                );
            }
            RefOr::Literal(FontFamilyValue::Single(_)) => {
                panic!("expected multiple font family values")
            }
            RefOr::Ref(_) => panic!("expected a literal font family value"),
        }
    }

    #[test]
    fn skips_when_array_contains_invalid_ref_pointer() {
        let value = json!(["Inter", { "$ref": "tokens/typography/body/fontFamily" }]);

        let (result, ctx) = parse_font_family(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidReference);
        assert_eq!(
            ctx.errors[0].message,
            "Invalid JSON pointer: tokens/typography/body/fontFamily"
        );
        assert_eq!(ctx.errors[0].path, "tokens.typography.body.fontFamily.1");
    }

    #[test]
    fn parses_top_level_ref_object() {
        let value = json!({ "$ref": "#/tokens/typography/brand/fontFamily" });

        let (result, ctx) = parse_font_family(&value);

        assert!(ctx.errors.is_empty());
        assert_eq!(
            expect_parsed(result),
            RefOr::Ref(JsonRef::new_local_pointer(
                "#/tokens/typography/brand/fontFamily".to_string(),
                JsonPointer::from("#/tokens/typography/brand/fontFamily"),
            ))
        );
    }

    #[test]
    fn parses_top_level_empty_string_ref_object() {
        let value = json!({ "$ref": "" });

        let (result, ctx) = parse_font_family(&value);

        assert!(ctx.errors.is_empty());
        assert_eq!(
            expect_parsed(result),
            RefOr::Ref(JsonRef::new_local_pointer(
                String::new(),
                JsonPointer::new(),
            ))
        );
    }

    #[test]
    fn skips_when_top_level_ref_pointer_is_invalid() {
        let value = json!({ "$ref": "tokens/typography/body/fontFamily" });

        let (result, ctx) = parse_font_family(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidReference);
        assert_eq!(
            ctx.errors[0].message,
            "Invalid JSON pointer: tokens/typography/body/fontFamily"
        );
        assert_eq!(ctx.errors[0].path, "tokens.typography.body.fontFamily");
    }

    #[test]
    fn skips_when_top_level_object_has_ref_and_extra_properties() {
        let value = json!({
            "$ref": "#/tokens/typography/brand/fontFamily",
            "fallback": "sans-serif"
        });

        let (result, ctx) = parse_font_family(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(
            ctx.errors[0].message,
            "Expected font family token value to be either a string or an array of strings, but found {\"$ref\":\"#/tokens/typography/brand/fontFamily\",\"fallback\":\"sans-serif\"}"
        );
        assert_eq!(ctx.errors[0].path, "tokens.typography.body.fontFamily");
    }

    #[test]
    fn skips_when_array_item_has_ref_and_extra_properties() {
        let value = json!([
            {
                "$ref": "#/tokens/typography/brand/fontFamily",
                "fallback": "sans-serif"
            }
        ]);

        let (result, ctx) = parse_font_family(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyType);
        assert_eq!(
            ctx.errors[0].message,
            "Expected a string, but found: {\"$ref\":\"#/tokens/typography/brand/fontFamily\",\"fallback\":\"sans-serif\"}"
        );
        assert_eq!(ctx.errors[0].path, "tokens.typography.body.fontFamily.0");
    }

    #[test]
    fn skips_when_array_contains_non_string_item() {
        let value = json!(["Inter", 42, "sans-serif"]);

        let (result, ctx) = parse_font_family(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyType);
        assert_eq!(ctx.errors[0].message, "Expected a string, but found: 42");
        assert_eq!(ctx.errors[0].path, "tokens.typography.body.fontFamily.1");
    }

    #[test]
    fn stops_at_first_invalid_array_item() {
        let value = json!([true, 42]);

        let (result, ctx) = parse_font_family(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].message, "Expected a string, but found: true");
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
                    "Expected font family token value to be either a string or an array of strings, but found {}",
                    value
                )
            );
        }
    }
}
