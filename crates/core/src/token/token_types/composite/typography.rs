//! The `typography` module defines the `TypographyTokenValue` struct which represents the DTCG typography token type.

use crate::{
    ParserContext,
    ir::{DiagnosticOwnership, JsonObject, ParseState, RefAliasOrLiteral, TryFromJson},
    token::token_types::{
        dimension::DimensionTokenValue, font_family::FontFamilyTokenValue,
        font_weight::FontWeightTokenValue, number::NumberTokenValue,
    },
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypographyFontFamily(pub RefAliasOrLiteral<FontFamilyTokenValue>);

impl<'a> TryFromJson<'a> for TypographyFontFamily {
    fn try_from_json(
        ctx: &mut ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match RefAliasOrLiteral::<FontFamilyTokenValue>::try_from_json(ctx, path, value) {
            ParseState::Parsed(val) => ParseState::Parsed(Self(val)),
            ParseState::Invalid(inv) => {
                if inv.ownership == DiagnosticOwnership::Silent {
                    ctx.push_to_errors(
                        crate::errors::DiagnosticCode::InvalidPropertyValue,
                        format!(
                            "Expected a DTCG reference, a JSON reference, or a font family token, but got {:?}",
                            value
                        ),
                        path.into(),
                    );
                    return ParseState::invalid_emitted(inv.reason);
                }
                ParseState::Invalid(inv)
            }
            ParseState::NoMatch => {
                ctx.push_to_errors(
                    crate::errors::DiagnosticCode::InvalidPropertyValue,
                    format!(
                        "Expected a DTCG reference, a JSON reference, or a font family token, but got {:?}",
                        value
                    ),
                    path.into(),
                );
                ParseState::NoMatch
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypographyFontSize(pub RefAliasOrLiteral<DimensionTokenValue>);

impl<'a> TryFromJson<'a> for TypographyFontSize {
    fn try_from_json(
        ctx: &mut ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match RefAliasOrLiteral::<DimensionTokenValue>::try_from_json(ctx, path, value) {
            ParseState::Parsed(val) => ParseState::Parsed(Self(val)),
            ParseState::Invalid(inv) => {
                if inv.ownership == DiagnosticOwnership::Silent {
                    ctx.push_to_errors(
                        crate::errors::DiagnosticCode::InvalidPropertyValue,
                        format!(
                            "Expected a DTCG reference, a JSON reference, or a dimension token, but got {:?}",
                            value
                        ),
                        path.into(),
                    );
                    return ParseState::invalid_emitted(inv.reason);
                }
                ParseState::Invalid(inv)
            }
            ParseState::NoMatch => {
                ctx.push_to_errors(
                    crate::errors::DiagnosticCode::InvalidPropertyValue,
                    format!(
                        "Expected a DTCG reference, a JSON reference, or a dimension token, but got {:?}",
                        value
                    ),
                    path.into(),
                );
                ParseState::NoMatch
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypographyFontWeight(pub RefAliasOrLiteral<FontWeightTokenValue>);

impl<'a> TryFromJson<'a> for TypographyFontWeight {
    fn try_from_json(
        ctx: &mut ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match RefAliasOrLiteral::<FontWeightTokenValue>::try_from_json(ctx, path, value) {
            ParseState::Parsed(val) => ParseState::Parsed(Self(val)),
            ParseState::Invalid(inv) => {
                if inv.ownership == DiagnosticOwnership::Silent {
                    ctx.push_to_errors(
                        crate::errors::DiagnosticCode::InvalidPropertyValue,
                        format!(
                            "Expected a DTCG reference, a JSON reference, or a font weight token, but got {:?}",
                            value
                        ),
                        path.into(),
                    );
                    return ParseState::invalid_emitted(inv.reason);
                }
                ParseState::Invalid(inv)
            }
            ParseState::NoMatch => {
                ctx.push_to_errors(
                    crate::errors::DiagnosticCode::InvalidPropertyValue,
                    format!(
                        "Expected a DTCG reference, a JSON reference, or a font weight token, but got {:?}",
                            value
                    ),
                    path.into(),
                );
                ParseState::NoMatch
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypographyLetterSpacing(pub RefAliasOrLiteral<DimensionTokenValue>);

impl<'a> TryFromJson<'a> for TypographyLetterSpacing {
    fn try_from_json(
        ctx: &mut ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match RefAliasOrLiteral::<DimensionTokenValue>::try_from_json(ctx, path, value) {
            ParseState::Parsed(val) => ParseState::Parsed(Self(val)),
            ParseState::Invalid(inv) => {
                if inv.ownership == DiagnosticOwnership::Silent {
                    ctx.push_to_errors(
                        crate::errors::DiagnosticCode::InvalidPropertyValue,
                        format!(
                            "Expected a DTCG reference, a JSON reference, or a dimension token, but got {:?}",
                            value
                        ),
                        path.into(),
                    );
                    return ParseState::invalid_emitted(inv.reason);
                }
                ParseState::Invalid(inv)
            }
            ParseState::NoMatch => {
                ctx.push_to_errors(
                    crate::errors::DiagnosticCode::InvalidPropertyValue,
                    format!(
                        "Expected a DTCG reference, a JSON reference, or a dimension token, but got {:?}",
                            value
                    ),
                    path.into(),
                );
                ParseState::NoMatch
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypographyLineHeight(pub RefAliasOrLiteral<NumberTokenValue>);

impl<'a> TryFromJson<'a> for TypographyLineHeight {
    fn try_from_json(
        ctx: &mut ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match RefAliasOrLiteral::<NumberTokenValue>::try_from_json(ctx, path, value) {
            ParseState::Parsed(val) => ParseState::Parsed(Self(val)),
            ParseState::Invalid(inv) => {
                if inv.ownership == DiagnosticOwnership::Silent {
                    ctx.push_to_errors(
                        crate::errors::DiagnosticCode::InvalidPropertyValue,
                        format!(
                            "Expected a DTCG reference, a JSON reference, or a number token, but got {:?}",
                            value
                        ),
                        path.into(),
                    );
                    return ParseState::invalid_emitted(inv.reason);
                }
                ParseState::Invalid(inv)
            }
            ParseState::NoMatch => {
                ctx.push_to_errors(
                    crate::errors::DiagnosticCode::InvalidPropertyValue,
                    format!(
                        "Expected a DTCG reference, a JSON reference, or a number token, but got {:?}",
                            value
                    ),
                    path.into(),
                );
                ParseState::NoMatch
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TypographyTokenValue {
    pub font_family: TypographyFontFamily,
    pub font_size: TypographyFontSize,
    pub font_weight: TypographyFontWeight,
    pub letter_spacing: TypographyLetterSpacing,
    pub line_height: TypographyLineHeight,
}

impl<'a> TryFromJson<'a> for TypographyTokenValue {
    fn try_from_json(
        ctx: &mut ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        let obj = match value {
            serde_json::Value::Object(map) => JsonObject(map),
            _ => return ParseState::NoMatch,
        };

        let font_family = obj.required_field::<TypographyFontFamily>(ctx, path, "fontFamily");
        let font_size = obj.required_field::<TypographyFontSize>(ctx, path, "fontSize");
        let font_weight = obj.required_field::<TypographyFontWeight>(ctx, path, "fontWeight");
        let letter_spacing =
            obj.required_field::<TypographyLetterSpacing>(ctx, path, "letterSpacing");
        let line_height = obj.required_field::<TypographyLineHeight>(ctx, path, "lineHeight");

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
            ) => ParseState::Parsed(Self {
                font_family,
                font_size,
                font_weight,
                letter_spacing,
                line_height,
            }),
            _ => ParseState::NoMatch, // If any required field is missing, we return NoMatch. The required_field method will have already emitted an error for the missing field.
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{
        ParserContext,
        errors::DiagnosticCode,
        ir::{ParseState, RefAliasOrLiteral, TryFromJson},
    };

    fn test_ctx() -> ParserContext {
        ParserContext::new("{}".to_string())
    }

    fn valid_typography_object() -> serde_json::Value {
        json!({
            "fontFamily": "Inter",
            "fontSize": { "value": 16, "unit": "px" },
            "fontWeight": 700,
            "letterSpacing": { "value": 0, "unit": "px" },
            "lineHeight": 1.5
        })
    }

    #[test]
    fn typography_font_family_parses_literal_ref_and_alias() {
        let mut ctx = test_ctx();

        let literal =
            TypographyFontFamily::try_from_json(&mut ctx, "#/token/fontFamily", &json!("Inter"));
        let by_ref = TypographyFontFamily::try_from_json(
            &mut ctx,
            "#/token/fontFamily",
            &json!({ "$ref": "#/typography/base/fontFamily" }),
        );
        let alias = TypographyFontFamily::try_from_json(
            &mut ctx,
            "#/token/fontFamily",
            &json!("{typography.base.fontFamily}"),
        );

        assert!(matches!(
            literal,
            ParseState::Parsed(TypographyFontFamily(RefAliasOrLiteral::Literal(_)))
        ));
        assert!(matches!(
            by_ref,
            ParseState::Parsed(TypographyFontFamily(RefAliasOrLiteral::Ref(_)))
        ));
        assert!(matches!(
            alias,
            ParseState::Parsed(TypographyFontFamily(RefAliasOrLiteral::Alias(_)))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn typography_font_size_parses_literal_ref_and_alias() {
        let mut ctx = test_ctx();

        let literal = TypographyFontSize::try_from_json(
            &mut ctx,
            "#/token/fontSize",
            &json!({ "value": 16, "unit": "px" }),
        );
        let by_ref = TypographyFontSize::try_from_json(
            &mut ctx,
            "#/token/fontSize",
            &json!({ "$ref": "#/typography/base/fontSize" }),
        );
        let alias = TypographyFontSize::try_from_json(
            &mut ctx,
            "#/token/fontSize",
            &json!("{typography.base.fontSize}"),
        );

        assert!(matches!(
            literal,
            ParseState::Parsed(TypographyFontSize(RefAliasOrLiteral::Literal(_)))
        ));
        assert!(matches!(
            by_ref,
            ParseState::Parsed(TypographyFontSize(RefAliasOrLiteral::Ref(_)))
        ));
        assert!(matches!(
            alias,
            ParseState::Parsed(TypographyFontSize(RefAliasOrLiteral::Alias(_)))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn typography_font_weight_parses_literal_ref_and_alias() {
        let mut ctx = test_ctx();

        let literal =
            TypographyFontWeight::try_from_json(&mut ctx, "#/token/fontWeight", &json!(600));
        let by_ref = TypographyFontWeight::try_from_json(
            &mut ctx,
            "#/token/fontWeight",
            &json!({ "$ref": "#/typography/base/fontWeight" }),
        );
        let alias = TypographyFontWeight::try_from_json(
            &mut ctx,
            "#/token/fontWeight",
            &json!("{typography.base.fontWeight}"),
        );

        assert!(matches!(
            literal,
            ParseState::Parsed(TypographyFontWeight(RefAliasOrLiteral::Literal(_)))
        ));
        assert!(matches!(
            by_ref,
            ParseState::Parsed(TypographyFontWeight(RefAliasOrLiteral::Ref(_)))
        ));
        assert!(matches!(
            alias,
            ParseState::Parsed(TypographyFontWeight(RefAliasOrLiteral::Alias(_)))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn typography_letter_spacing_parses_literal_ref_and_alias() {
        let mut ctx = test_ctx();

        let literal = TypographyLetterSpacing::try_from_json(
            &mut ctx,
            "#/token/letterSpacing",
            &json!({ "value": 1, "unit": "px" }),
        );
        let by_ref = TypographyLetterSpacing::try_from_json(
            &mut ctx,
            "#/token/letterSpacing",
            &json!({ "$ref": "#/typography/base/letterSpacing" }),
        );
        let alias = TypographyLetterSpacing::try_from_json(
            &mut ctx,
            "#/token/letterSpacing",
            &json!("{typography.base.letterSpacing}"),
        );

        assert!(matches!(
            literal,
            ParseState::Parsed(TypographyLetterSpacing(RefAliasOrLiteral::Literal(_)))
        ));
        assert!(matches!(
            by_ref,
            ParseState::Parsed(TypographyLetterSpacing(RefAliasOrLiteral::Ref(_)))
        ));
        assert!(matches!(
            alias,
            ParseState::Parsed(TypographyLetterSpacing(RefAliasOrLiteral::Alias(_)))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn typography_line_height_parses_literal_ref_and_alias() {
        let mut ctx = test_ctx();

        let literal =
            TypographyLineHeight::try_from_json(&mut ctx, "#/token/lineHeight", &json!(1.5));
        let by_ref = TypographyLineHeight::try_from_json(
            &mut ctx,
            "#/token/lineHeight",
            &json!({ "$ref": "#/typography/base/lineHeight" }),
        );
        let alias = TypographyLineHeight::try_from_json(
            &mut ctx,
            "#/token/lineHeight",
            &json!("{typography.base.lineHeight}"),
        );

        assert!(matches!(
            literal,
            ParseState::Parsed(TypographyLineHeight(RefAliasOrLiteral::Literal(_)))
        ));
        assert!(matches!(
            by_ref,
            ParseState::Parsed(TypographyLineHeight(RefAliasOrLiteral::Ref(_)))
        ));
        assert!(matches!(
            alias,
            ParseState::Parsed(TypographyLineHeight(RefAliasOrLiteral::Alias(_)))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn typography_token_value_parses_valid_object() {
        let mut ctx = test_ctx();

        let state =
            TypographyTokenValue::try_from_json(&mut ctx, "#/token", &valid_typography_object());

        assert!(matches!(state, ParseState::Parsed(_)));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn typography_token_value_returns_no_match_for_non_object() {
        let mut ctx = test_ctx();

        let state = TypographyTokenValue::try_from_json(&mut ctx, "#/token", &json!(42));

        assert!(matches!(state, ParseState::NoMatch));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn typography_token_value_reports_missing_required_field() {
        let mut ctx = test_ctx();
        let input = json!({
            "fontFamily": "Inter",
            "fontSize": { "value": 16, "unit": "px" },
            "fontWeight": 700,
            "letterSpacing": { "value": 0, "unit": "px" }
        });

        let state = TypographyTokenValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::NoMatch));
        assert!(ctx.errors.iter().any(|e| {
            e.code == DiagnosticCode::MissingRequiredProperty && e.path == "#/token/lineHeight"
        }));
    }

    #[test]
    fn typography_token_value_reports_invalid_field_value() {
        let mut ctx = test_ctx();
        let input = json!({
            "fontFamily": true,
            "fontSize": { "value": 16, "unit": "px" },
            "fontWeight": 700,
            "letterSpacing": { "value": 0, "unit": "px" },
            "lineHeight": 1.5
        });

        let state = TypographyTokenValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::NoMatch));
        assert!(ctx.errors.iter().any(|e| e.path == "#/token/fontFamily"));
    }
}
