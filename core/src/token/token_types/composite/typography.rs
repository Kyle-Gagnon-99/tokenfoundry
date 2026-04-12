//! The `typography` module defines the typography token type per the DTCG specification.

use crate::{
    ir::{TryFromJson, require_object},
    token::token_types::{
        composite::RefAliasOrLiteral, dimension::DimensionTokenValue,
        font_family::FontFamilyTokenValue, font_weight::FontWeightTokenValue,
        number::NumberTokenValue,
    },
};

#[derive(Debug, Clone, PartialEq)]
pub struct TypographyFontFamily(pub FontFamilyTokenValue);

impl<'a> TryFromJson<'a> for TypographyFontFamily {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        FontFamilyTokenValue::try_from_json(ctx, path, value).map(TypographyFontFamily)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypographyFontSize(pub DimensionTokenValue);

impl<'a> TryFromJson<'a> for TypographyFontSize {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        DimensionTokenValue::try_from_json(ctx, path, value).map(TypographyFontSize)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypographyFontWeight(pub FontWeightTokenValue);

impl<'a> TryFromJson<'a> for TypographyFontWeight {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        FontWeightTokenValue::try_from_json(ctx, path, value).map(TypographyFontWeight)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypographyLetterSpacing(pub DimensionTokenValue);

impl<'a> TryFromJson<'a> for TypographyLetterSpacing {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        DimensionTokenValue::try_from_json(ctx, path, value).map(TypographyLetterSpacing)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypographyLineHeight(pub NumberTokenValue);

impl<'a> TryFromJson<'a> for TypographyLineHeight {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        NumberTokenValue::try_from_json(ctx, path, value).map(TypographyLineHeight)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypographyTokenValue {
    pub font_family: RefAliasOrLiteral<TypographyFontFamily>,
    pub font_size: RefAliasOrLiteral<TypographyFontSize>,
    pub font_weight: RefAliasOrLiteral<TypographyFontWeight>,
    pub letter_spacing: RefAliasOrLiteral<TypographyLetterSpacing>,
    pub line_height: RefAliasOrLiteral<TypographyLineHeight>,
}

impl<'a> TryFromJson<'a> for TypographyTokenValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        let obj = require_object(ctx, path, value, "typography token")?;
        let font_family =
            obj.required_field::<RefAliasOrLiteral<TypographyFontFamily>>(ctx, path, "fontFamily");
        let font_size =
            obj.required_field::<RefAliasOrLiteral<TypographyFontSize>>(ctx, path, "fontSize");
        let font_weight =
            obj.required_field::<RefAliasOrLiteral<TypographyFontWeight>>(ctx, path, "fontWeight");
        let letter_spacing = obj.required_field::<RefAliasOrLiteral<TypographyLetterSpacing>>(
            ctx,
            path,
            "letterSpacing",
        );
        let line_height =
            obj.required_field::<RefAliasOrLiteral<TypographyLineHeight>>(ctx, path, "lineHeight");

        match (
            font_family,
            font_size,
            font_weight,
            letter_spacing,
            line_height,
        ) {
            (
                Some(font_family),
                Some(font_size),
                Some(font_weight),
                Some(letter_spacing),
                Some(line_height),
            ) => Some(TypographyTokenValue {
                font_family,
                font_size,
                font_weight,
                letter_spacing,
                line_height,
            }),
            _ => None,
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
        token::token_types::composite::RefAliasOrLiteral,
    };
    use serde_json::json;

    fn parser_context() -> ParserContext {
        ParserContext::new("tests.json".into(), FileFormat::Json, String::new())
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum FieldMode {
        Literal,
        Alias,
        Ref,
    }

    fn font_family_json(mode: FieldMode) -> serde_json::Value {
        match mode {
            FieldMode::Literal => json!("Inter"),
            FieldMode::Alias => json!("{typography.font.family.base}"),
            FieldMode::Ref => json!({ "$ref": "#/typography/fontFamily" }),
        }
    }

    fn font_size_json(mode: FieldMode) -> serde_json::Value {
        match mode {
            FieldMode::Literal => json!({ "value": 16, "unit": "px" }),
            FieldMode::Alias => json!("{typography.font.size.md}"),
            FieldMode::Ref => json!({ "$ref": "#/typography/fontSize" }),
        }
    }

    fn font_weight_json(mode: FieldMode) -> serde_json::Value {
        match mode {
            FieldMode::Literal => json!("bold"),
            FieldMode::Alias => json!("{typography.font.weight.strong}"),
            FieldMode::Ref => json!({ "$ref": "#/typography/fontWeight" }),
        }
    }

    fn letter_spacing_json(mode: FieldMode) -> serde_json::Value {
        match mode {
            FieldMode::Literal => json!({ "value": 0, "unit": "px" }),
            FieldMode::Alias => json!("{typography.letterSpacing.normal}"),
            FieldMode::Ref => json!({ "$ref": "#/typography/letterSpacing" }),
        }
    }

    fn line_height_json(mode: FieldMode) -> serde_json::Value {
        match mode {
            FieldMode::Literal => json!(24),
            FieldMode::Alias => json!("{typography.lineHeight.body}"),
            FieldMode::Ref => json!({ "$ref": "#/typography/lineHeight" }),
        }
    }

    fn classify_font_family(value: &RefAliasOrLiteral<TypographyFontFamily>) -> FieldMode {
        match value {
            RefAliasOrLiteral::Ref(_) => FieldMode::Ref,
            RefAliasOrLiteral::Alias(_) => FieldMode::Alias,
            RefAliasOrLiteral::Literal(_) => FieldMode::Literal,
        }
    }

    fn classify_font_size(value: &RefAliasOrLiteral<TypographyFontSize>) -> FieldMode {
        match value {
            RefAliasOrLiteral::Ref(_) => FieldMode::Ref,
            RefAliasOrLiteral::Alias(_) => FieldMode::Alias,
            RefAliasOrLiteral::Literal(_) => FieldMode::Literal,
        }
    }

    fn classify_font_weight(value: &RefAliasOrLiteral<TypographyFontWeight>) -> FieldMode {
        match value {
            RefAliasOrLiteral::Ref(_) => FieldMode::Ref,
            RefAliasOrLiteral::Alias(_) => FieldMode::Alias,
            RefAliasOrLiteral::Literal(_) => FieldMode::Literal,
        }
    }

    fn classify_letter_spacing(value: &RefAliasOrLiteral<TypographyLetterSpacing>) -> FieldMode {
        match value {
            RefAliasOrLiteral::Ref(_) => FieldMode::Ref,
            RefAliasOrLiteral::Alias(_) => FieldMode::Alias,
            RefAliasOrLiteral::Literal(_) => FieldMode::Literal,
        }
    }

    fn classify_line_height(value: &RefAliasOrLiteral<TypographyLineHeight>) -> FieldMode {
        match value {
            RefAliasOrLiteral::Ref(_) => FieldMode::Ref,
            RefAliasOrLiteral::Alias(_) => FieldMode::Alias,
            RefAliasOrLiteral::Literal(_) => FieldMode::Literal,
        }
    }

    #[test]
    fn parses_typography_wrapped_fields_as_literals() {
        let mut ctx = parser_context();

        let family = TypographyFontFamily::try_from_json(
            &mut ctx,
            "/token/fontFamily",
            &font_family_json(FieldMode::Literal),
        );
        let size = TypographyFontSize::try_from_json(
            &mut ctx,
            "/token/fontSize",
            &font_size_json(FieldMode::Literal),
        );
        let weight = TypographyFontWeight::try_from_json(
            &mut ctx,
            "/token/fontWeight",
            &font_weight_json(FieldMode::Literal),
        );

        assert!(matches!(family, Some(TypographyFontFamily(_))));
        assert!(matches!(size, Some(TypographyFontSize(_))));
        assert!(matches!(weight, Some(TypographyFontWeight(_))));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn rejects_invalid_typography_wrapped_fields() {
        let mut ctx = parser_context();

        let family = TypographyFontFamily::try_from_json(&mut ctx, "/token/fontFamily", &json!(42));
        let size = TypographyFontSize::try_from_json(&mut ctx, "/token/fontSize", &json!("large"));
        let weight =
            TypographyFontWeight::try_from_json(&mut ctx, "/token/fontWeight", &json!(true));
        let line_height =
            TypographyLineHeight::try_from_json(&mut ctx, "/token/lineHeight", &json!("tall"));

        assert_eq!(family, None);
        assert_eq!(size, None);
        assert_eq!(weight, None);
        assert_eq!(line_height, None);
        assert_eq!(ctx.errors.len(), 4);
        assert_eq!(ctx.errors[0].path, "/token/fontFamily");
        assert_eq!(ctx.errors[1].path, "/token/fontSize");
        assert_eq!(ctx.errors[2].path, "/token/fontWeight");
        assert_eq!(ctx.errors[3].path, "/token/lineHeight");
    }

    #[test]
    fn parses_typography_token_with_literal_fields() {
        let mut ctx = parser_context();
        let value = json!({
            "fontFamily": font_family_json(FieldMode::Literal),
            "fontSize": font_size_json(FieldMode::Literal),
            "fontWeight": font_weight_json(FieldMode::Literal),
            "letterSpacing": letter_spacing_json(FieldMode::Literal),
            "lineHeight": line_height_json(FieldMode::Literal)
        });

        let parsed = TypographyTokenValue::try_from_json(&mut ctx, "/token", &value);

        assert!(matches!(
            parsed,
            Some(TypographyTokenValue {
                font_family: RefAliasOrLiteral::Literal(_),
                font_size: RefAliasOrLiteral::Literal(_),
                font_weight: RefAliasOrLiteral::Literal(_),
                letter_spacing: RefAliasOrLiteral::Literal(_),
                line_height: RefAliasOrLiteral::Literal(_),
            })
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn parses_typography_token_with_ref_fields() {
        let mut ctx = parser_context();
        let value = json!({
            "fontFamily": font_family_json(FieldMode::Ref),
            "fontSize": font_size_json(FieldMode::Ref),
            "fontWeight": font_weight_json(FieldMode::Ref),
            "letterSpacing": letter_spacing_json(FieldMode::Ref),
            "lineHeight": line_height_json(FieldMode::Ref)
        });

        let parsed = TypographyTokenValue::try_from_json(&mut ctx, "/token", &value).unwrap();

        assert_eq!(
            parsed.font_family,
            RefAliasOrLiteral::Ref(JsonRef::new_local_pointer(
                "#/typography/fontFamily".into(),
                JsonPointer::from("#/typography/fontFamily"),
            ))
        );
        assert_eq!(
            parsed.font_size,
            RefAliasOrLiteral::Ref(JsonRef::new_local_pointer(
                "#/typography/fontSize".into(),
                JsonPointer::from("#/typography/fontSize"),
            ))
        );
        assert_eq!(
            parsed.font_weight,
            RefAliasOrLiteral::Ref(JsonRef::new_local_pointer(
                "#/typography/fontWeight".into(),
                JsonPointer::from("#/typography/fontWeight"),
            ))
        );
        assert_eq!(
            parsed.letter_spacing,
            RefAliasOrLiteral::Ref(JsonRef::new_local_pointer(
                "#/typography/letterSpacing".into(),
                JsonPointer::from("#/typography/letterSpacing"),
            ))
        );
        assert_eq!(
            parsed.line_height,
            RefAliasOrLiteral::Ref(JsonRef::new_local_pointer(
                "#/typography/lineHeight".into(),
                JsonPointer::from("#/typography/lineHeight"),
            ))
        );
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn parses_all_typography_field_mode_combinations() {
        let modes = [FieldMode::Literal, FieldMode::Alias, FieldMode::Ref];

        for family_mode in modes {
            for size_mode in modes {
                for weight_mode in modes {
                    for letter_spacing_mode in modes {
                        for line_height_mode in modes {
                            let mut ctx = parser_context();
                            let value = json!({
                                "fontFamily": font_family_json(family_mode),
                                "fontSize": font_size_json(size_mode),
                                "fontWeight": font_weight_json(weight_mode),
                                "letterSpacing": letter_spacing_json(letter_spacing_mode),
                                "lineHeight": line_height_json(line_height_mode)
                            });

                            let parsed = TypographyTokenValue::try_from_json(&mut ctx, "/token", &value)
                                .unwrap_or_else(|| {
                                    panic!(
                                        "expected typography token to parse for combination {:?}/{:?}/{:?}/{:?}/{:?}; errors: {:?}",
                                        family_mode,
                                        size_mode,
                                        weight_mode,
                                        letter_spacing_mode,
                                        line_height_mode,
                                        ctx.errors
                                    )
                                });

                            assert_eq!(classify_font_family(&parsed.font_family), family_mode);
                            assert_eq!(classify_font_size(&parsed.font_size), size_mode);
                            assert_eq!(classify_font_weight(&parsed.font_weight), weight_mode);
                            assert_eq!(
                                classify_letter_spacing(&parsed.letter_spacing),
                                letter_spacing_mode
                            );
                            assert_eq!(classify_line_height(&parsed.line_height), line_height_mode);
                            assert!(ctx.errors.is_empty());
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn reports_missing_required_typography_fields_without_duplicate_pass() {
        let mut ctx = parser_context();

        let parsed = TypographyTokenValue::try_from_json(&mut ctx, "/token", &json!({}));

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 5);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[0].path, "/token/fontFamily");
        assert_eq!(ctx.errors[1].path, "/token/fontSize");
        assert_eq!(ctx.errors[2].path, "/token/fontWeight");
        assert_eq!(ctx.errors[3].path, "/token/letterSpacing");
        assert_eq!(ctx.errors[4].path, "/token/lineHeight");
    }

    #[test]
    fn reports_invalid_typography_field_and_missing_peers_without_duplicate_pass() {
        let mut ctx = parser_context();

        let parsed = TypographyTokenValue::try_from_json(
            &mut ctx,
            "/token",
            &json!({
                "fontFamily": 42
            }),
        );

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 5);
        assert_eq!(ctx.errors[0].path, "/token/fontFamily");
        assert_eq!(ctx.errors[1].path, "/token/fontSize");
        assert_eq!(ctx.errors[2].path, "/token/fontWeight");
        assert_eq!(ctx.errors[3].path, "/token/letterSpacing");
        assert_eq!(ctx.errors[4].path, "/token/lineHeight");
    }

    #[test]
    fn reports_invalid_top_level_shape_for_typography_token() {
        let mut ctx = parser_context();

        let parsed = TypographyTokenValue::try_from_json(&mut ctx, "/token", &json!("headline"));

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyType);
        assert_eq!(ctx.errors[0].path, "/token");
    }
}
