//! The `font_family` module defines the data structures for font family tokens defined in the DTCG specification,
//! which represents a font family in the UI, such as the name of the font and its fallback fonts.

use crate::ir::{RefOr, TryFromJson, parse_ref_or_value};

#[derive(Debug, Clone, PartialEq)]
pub struct FontFamilySingleValue(pub String);

impl<'a> TryFromJson<'a> for FontFamilySingleValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        match value {
            serde_json::Value::String(str_val) => Some(FontFamilySingleValue(str_val.clone())),
            _ => {
                ctx.push_to_errors(
                    crate::errors::DiagnosticCode::InvalidTokenValue,
                    format!(
                        "Expected font family token value to be a string, but found {}",
                        value
                    ),
                    path.into(),
                );
                None
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FontFamilyValue {
    Single(RefOr<FontFamilySingleValue>),
    Multiple(Vec<RefOr<FontFamilySingleValue>>),
}

impl<'a> TryFromJson<'a> for FontFamilyValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        match value {
            serde_json::Value::String(str_val) => Some(FontFamilyValue::Single(RefOr::Literal(
                FontFamilySingleValue(str_val.clone()),
            ))),
            serde_json::Value::Array(arr_val) => {
                let mut font_families = Vec::new();
                for (index, item) in arr_val.iter().enumerate() {
                    let val = parse_ref_or_value::<FontFamilySingleValue>(
                        ctx,
                        &format!("{}/{}", path, index),
                        item,
                    );
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
    ) -> Option<Self> {
        parse_ref_or_value(ctx, path, value).map(FontFamilyTokenValue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        FileFormat, ParserContext,
        errors::DiagnosticCode,
        ir::{RefOr, TryFromJson},
    };
    use serde_json::json;

    fn parser_context() -> ParserContext {
        ParserContext::new("tests.json".into(), FileFormat::Json, String::new())
    }

    #[test]
    fn parses_single_font_family_string() {
        let mut ctx = parser_context();

        let parsed = FontFamilySingleValue::try_from_json(&mut ctx, "/token", &json!("Inter"));

        assert_eq!(parsed, Some(FontFamilySingleValue("Inter".into())));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn rejects_non_string_single_font_family_value() {
        let mut ctx = parser_context();

        let parsed = FontFamilySingleValue::try_from_json(&mut ctx, "/token", &json!(16));

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(ctx.errors[0].path, "/token");
    }

    #[test]
    fn parses_font_family_value_from_single_string() {
        let mut ctx = parser_context();

        let parsed = FontFamilyValue::try_from_json(&mut ctx, "/token", &json!("Inter"));

        assert_eq!(
            parsed,
            Some(FontFamilyValue::Single(RefOr::Literal(
                FontFamilySingleValue("Inter".into())
            )))
        );
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn parses_font_family_value_from_empty_array() {
        let mut ctx = parser_context();

        let parsed = FontFamilyValue::try_from_json(&mut ctx, "/token", &json!([]));

        assert_eq!(parsed, Some(FontFamilyValue::Multiple(Vec::new())));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn parses_font_family_value_from_string_array_and_references() {
        let mut ctx = parser_context();
        let value = json!([
            "Inter",
            { "$ref": "#/fonts/fallback" },
            "sans-serif"
        ]);

        let parsed = FontFamilyValue::try_from_json(&mut ctx, "/token", &value);

        assert_eq!(
            parsed,
            Some(FontFamilyValue::Multiple(vec![
                RefOr::Literal(FontFamilySingleValue("Inter".into())),
                RefOr::from_ref(crate::ir::JsonRef::new_local_pointer(
                    "#/fonts/fallback".into(),
                    crate::ir::JsonPointer::from("#/fonts/fallback"),
                )),
                RefOr::Literal(FontFamilySingleValue("sans-serif".into())),
            ]))
        );
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn rejects_font_family_value_with_invalid_top_level_type() {
        let mut ctx = parser_context();

        let parsed = FontFamilyValue::try_from_json(&mut ctx, "/token", &json!(true));

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(ctx.errors[0].path, "/token");
    }

    #[test]
    fn rejects_font_family_array_when_item_has_invalid_type() {
        let mut ctx = parser_context();
        let value = json!(["Inter", 42, "sans-serif"]);

        let parsed = FontFamilyValue::try_from_json(&mut ctx, "/token", &value);

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(ctx.errors[0].path, "/token/1");
    }

    #[test]
    fn rejects_font_family_array_when_reference_is_invalid() {
        let mut ctx = parser_context();
        let value = json!([
            "Inter",
            { "$ref": "not-a-pointer" }
        ]);

        let parsed = FontFamilyValue::try_from_json(&mut ctx, "/token", &value);

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidReference);
        assert_eq!(ctx.errors[0].path, "/token/1");
    }

    #[test]
    fn parses_font_family_token_value_as_literal_string() {
        let mut ctx = parser_context();

        let parsed = FontFamilyTokenValue::try_from_json(&mut ctx, "/token", &json!("Inter"));

        assert_eq!(
            parsed,
            Some(FontFamilyTokenValue(RefOr::Literal(
                FontFamilyValue::Single(RefOr::Literal(FontFamilySingleValue("Inter".into())),)
            )))
        );
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn parses_font_family_token_value_as_top_level_reference() {
        let mut ctx = parser_context();

        let parsed = FontFamilyTokenValue::try_from_json(
            &mut ctx,
            "/token",
            &json!({ "$ref": "#/fonts/body" }),
        );

        assert_eq!(
            parsed,
            Some(FontFamilyTokenValue(RefOr::from_ref(
                crate::ir::JsonRef::new_local_pointer(
                    "#/fonts/body".into(),
                    crate::ir::JsonPointer::from("#/fonts/body"),
                ),
            )))
        );
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn rejects_font_family_token_value_with_invalid_top_level_reference() {
        let mut ctx = parser_context();

        let parsed = FontFamilyTokenValue::try_from_json(
            &mut ctx,
            "/token",
            &json!({ "$ref": "not-a-pointer" }),
        );

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidReference);
        assert_eq!(ctx.errors[0].path, "/token");
    }

    #[test]
    fn rejects_font_family_token_value_with_invalid_object_shape() {
        let mut ctx = parser_context();

        let parsed =
            FontFamilyTokenValue::try_from_json(&mut ctx, "/token", &json!({ "primary": "Inter" }));

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(ctx.errors[0].path, "/token");
    }
}
