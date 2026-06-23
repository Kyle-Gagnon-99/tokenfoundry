//! The `color` module defines the `ColorTokenValue` struct which represents the DTCG color token type.

use crate::{
    errors::DiagnosticCode,
    ir::{InvalidReason, JsonArray, JsonNumber, JsonObject, ParseState, RefOrLiteral, TryFromJson},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ColorComponentArrayElement {
    Number(JsonNumber),
    None,
}

impl<'a> TryFromJson<'a> for ColorComponentArrayElement {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &JsonPointer,
        value: &'a serde_json::Value,
    ) -> crate::ir::ParseState<Self> {
        match value {
            serde_json::Value::Number(num) => {
                ParseState::Parsed(ColorComponentArrayElement::Number(JsonNumber(num.clone())))
            }
            serde_json::Value::String(s) if s == "none" => {
                ParseState::Parsed(ColorComponentArrayElement::None)
            }
            _ => {
                ctx.push_to_errors(
                    crate::errors::DiagnosticCode::InvalidPropertyValue,
                    format!(
                        "Expected either a number or 'none' for color component at {}",
                        path
                    ),
                    path.into(),
                );
                ParseState::invalid_emitted(InvalidReason::InvalidValue)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ColorComponentArray(pub [ColorComponentArrayElement; 3]);

impl<'a> TryFromJson<'a> for ColorComponentArray {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &JsonPointer,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        let array = match value {
            serde_json::Value::Array(arr) => JsonArray(arr),
            _ => return ParseState::NoMatch,
        };

        if array.len() != 3 {
            ctx.push_to_errors(
                crate::errors::DiagnosticCode::InvalidPropertyValue,
                format!(
                    "Expected an array of 3 elements for color component at {}",
                    path
                ),
                path.into(),
            );
            return ParseState::invalid_emitted(InvalidReason::WrongShape);
        }

        let result = match array.parse_for_each::<ColorComponentArrayElement>(ctx, path) {
            Some(res) => res,
            None => {
                ctx.push_to_errors(
                    crate::errors::DiagnosticCode::InvalidPropertyValue,
                    format!(
                        "Expected all entries to be either numbers or 'none' for color component at {}",
                        path
                    ),
                    path.into(),
                );
                return ParseState::invalid_emitted(InvalidReason::InvalidValue);
            }
        };

        // Since we validated the length, we can safely convert the Vec to an array
        ParseState::Parsed(ColorComponentArray(result.try_into().unwrap()))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ColorSpaceString {
    SRGB,
    SRGBLinear,
    HSL,
    HWB,
    CIELAB,
    LCH,
    OKLAB,
    OKLCH,
    DisplayP3,
    A98RGB,
    ProPhotoRGB,
    Rec2020,
    XYZD65,
    XYZD50,
}

impl<'a> TryFromJson<'a> for ColorSpaceString {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &JsonPointer,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match value {
            serde_json::Value::String(s) => match s.as_str() {
                "srgb" => ParseState::Parsed(Self::SRGB),
                "srgb-linear" => ParseState::Parsed(Self::SRGBLinear),
                "hsl" => ParseState::Parsed(Self::HSL),
                "hwb" => ParseState::Parsed(Self::HWB),
                "cielab" => ParseState::Parsed(Self::CIELAB),
                "lch" => ParseState::Parsed(Self::LCH),
                "oklab" => ParseState::Parsed(Self::OKLAB),
                "oklch" => ParseState::Parsed(Self::OKLCH),
                "display-p3" => ParseState::Parsed(Self::DisplayP3),
                "a98rgb" => ParseState::Parsed(Self::A98RGB),
                "prophoto-rgb" => ParseState::Parsed(Self::ProPhotoRGB),
                "rec2020" => ParseState::Parsed(Self::Rec2020),
                "xyz-d65" => ParseState::Parsed(Self::XYZD65),
                "xyz-d50" => ParseState::Parsed(Self::XYZD50),
                _ => {
                    ctx.push_to_errors(
                        crate::errors::DiagnosticCode::InvalidPropertyValue,
                        format!("Unexpected color space string '{}' at {}", s, path),
                        path.into(),
                    );
                    ParseState::invalid_emitted(InvalidReason::InvalidValue)
                }
            },
            _ => {
                ctx.push_to_errors(
                    crate::errors::DiagnosticCode::InvalidPropertyValue,
                    format!(
                        "Expected a string for color space at {}, but got {:?}",
                        path, value
                    ),
                    path.into(),
                );
                ParseState::invalid_emitted(InvalidReason::InvalidValue)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ColorAlpha(pub JsonNumber);

impl<'a> TryFromJson<'a> for ColorAlpha {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &JsonPointer,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match JsonNumber::try_from_json(ctx, path, value) {
            ParseState::Parsed(number) => ParseState::Parsed(Self(number)),
            _ => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidPropertyValue,
                    format!(
                        "Expected a number for color alpha at {}, but got {:?}",
                        path, value
                    ),
                    path.into(),
                );
                ParseState::invalid_emitted(InvalidReason::InvalidValue)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ColorHexString(pub String);

impl<'a> TryFromJson<'a> for ColorHexString {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &JsonPointer,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match value {
            serde_json::Value::String(s) => ParseState::Parsed(ColorHexString(s.clone())),
            _ => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidPropertyValue,
                    format!(
                        "Expected a string for color hex value at {}, but got {:?}",
                        path, value
                    ),
                    path.into(),
                );
                ParseState::invalid_emitted(InvalidReason::InvalidValue)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ColorTokenValue {
    pub color_space: RefOrLiteral<ColorSpaceString>,
    pub components: RefOrLiteral<ColorComponentArray>,
    pub alpha: Option<RefOrLiteral<ColorAlpha>>,
    pub hex: Option<RefOrLiteral<ColorHexString>>,
}

impl<'a> TryFromJson<'a> for ColorTokenValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &JsonPointer,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        let obj = match value {
            serde_json::Value::Object(map) => JsonObject::new(map),
            _ => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidPropertyValue,
                    format!(
                        "Expected an object for color token at {}, but got {:?}",
                        path, value
                    ),
                    path.into(),
                );
                return ParseState::invalid_emitted(InvalidReason::InvalidValue);
            }
        };

        let color_space = obj.required_field_any::<RefOrLiteral<ColorSpaceString>>(
            ctx,
            path,
            &["colorSpace", "color_space"],
        );
        let components =
            obj.required_field::<RefOrLiteral<ColorComponentArray>>(ctx, path, "components");
        let alpha = obj.optional_field::<RefOrLiteral<ColorAlpha>>(ctx, path, "alpha");
        let hex = obj.optional_field::<RefOrLiteral<ColorHexString>>(ctx, path, "hex");

        match (color_space, components) {
            (Some(color_space), Some(components)) => ParseState::Parsed(Self {
                color_space,
                components,
                alpha,
                hex,
            }),
            _ => ParseState::invalid_emitted(InvalidReason::InvalidValue),
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
        ir::{JsonNumber, ParseState, RefOrLiteral, TryFromJson},
    };

    fn test_ctx() -> ParserContext {
        ParserContext::new("{}".to_string())
    }

    // ── ColorComponentArrayElement ────────────────────────────────────────

    #[test]
    fn color_component_element_parses_number() {
        let mut ctx = test_ctx();

        let state = ColorComponentArrayElement::try_from_json(&mut ctx, "#/token/0", &json!(0.5));

        assert!(matches!(
            state,
            ParseState::Parsed(ColorComponentArrayElement::Number(_))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn color_component_element_parses_none_string() {
        let mut ctx = test_ctx();

        let state =
            ColorComponentArrayElement::try_from_json(&mut ctx, "#/token/0", &json!("none"));

        assert!(matches!(
            state,
            ParseState::Parsed(ColorComponentArrayElement::None)
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn color_component_element_rejects_non_none_strings() {
        let mut ctx = test_ctx();

        let state = ColorComponentArrayElement::try_from_json(&mut ctx, "#/token/0", &json!("red"));

        assert!(matches!(state, ParseState::Failed(_)));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyValue);
        assert_eq!(ctx.errors[0].path, "#/token/0");
    }

    #[test]
    fn color_component_element_rejects_other_types() {
        let invalid_inputs = [json!(true), json!([1, 2])];

        for input in invalid_inputs {
            let mut ctx = test_ctx();
            let state = ColorComponentArrayElement::try_from_json(&mut ctx, "#/token/0", &input);
            assert!(matches!(state, ParseState::Failed(_)));
            assert_eq!(ctx.errors.len(), 1);
            assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyValue);
        }
    }

    // ── ColorComponentArray ───────────────────────────────────────────────

    #[test]
    fn color_component_array_parses_three_numbers() {
        let mut ctx = test_ctx();
        let input = json!([0.1, 0.5, 0.9]);

        let state = ColorComponentArray::try_from_json(&mut ctx, "#/token", &input);

        let parsed = match state {
            ParseState::Parsed(v) => v,
            _ => panic!("expected parsed color component array"),
        };

        assert!(matches!(parsed.0[0], ColorComponentArrayElement::Number(_)));
        assert!(matches!(parsed.0[1], ColorComponentArrayElement::Number(_)));
        assert!(matches!(parsed.0[2], ColorComponentArrayElement::Number(_)));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn color_component_array_parses_mixed_numbers_and_none() {
        let mut ctx = test_ctx();
        let input = json!([120, "none", 0.8]);

        let state = ColorComponentArray::try_from_json(&mut ctx, "#/token", &input);

        let parsed = match state {
            ParseState::Parsed(v) => v,
            _ => panic!("expected parsed color component array"),
        };

        assert!(matches!(parsed.0[0], ColorComponentArrayElement::Number(_)));
        assert!(matches!(parsed.0[1], ColorComponentArrayElement::None));
        assert!(matches!(parsed.0[2], ColorComponentArrayElement::Number(_)));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn color_component_array_returns_no_match_for_non_array() {
        let mut ctx = test_ctx();

        let state = ColorComponentArray::try_from_json(&mut ctx, "#/token", &json!("bad"));

        assert!(matches!(state, ParseState::NoMatch));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn color_component_array_rejects_wrong_length() {
        let mut ctx = test_ctx();

        let state = ColorComponentArray::try_from_json(&mut ctx, "#/token", &json!([1, 2]));

        assert!(matches!(state, ParseState::Failed(_)));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyValue);
        assert_eq!(ctx.errors[0].path, "#/token");
    }

    #[test]
    fn color_component_array_rejects_invalid_element() {
        let mut ctx = test_ctx();
        let input = json!([1, "bad", 0.5]);

        let state = ColorComponentArray::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::Failed(_)));
        assert!(
            ctx.errors.iter().any(|e| {
                e.code == DiagnosticCode::InvalidPropertyType && e.path == "#/token/1"
            })
        );
        assert!(
            ctx.errors
                .iter()
                .any(|e| { e.code == DiagnosticCode::InvalidPropertyValue && e.path == "#/token" })
        );
    }

    // ── ColorSpaceString ──────────────────────────────────────────────────

    #[test]
    fn color_space_string_parses_all_known_values() {
        let cases = [
            ("srgb", ColorSpaceString::SRGB),
            ("srgb-linear", ColorSpaceString::SRGBLinear),
            ("hsl", ColorSpaceString::HSL),
            ("hwb", ColorSpaceString::HWB),
            ("cielab", ColorSpaceString::CIELAB),
            ("lch", ColorSpaceString::LCH),
            ("oklab", ColorSpaceString::OKLAB),
            ("oklch", ColorSpaceString::OKLCH),
            ("display-p3", ColorSpaceString::DisplayP3),
            ("a98rgb", ColorSpaceString::A98RGB),
            ("prophoto-rgb", ColorSpaceString::ProPhotoRGB),
            ("rec2020", ColorSpaceString::Rec2020),
            ("xyz-d65", ColorSpaceString::XYZD65),
            ("xyz-d50", ColorSpaceString::XYZD50),
        ];

        for (input, expected) in cases {
            let mut ctx = test_ctx();
            let state = ColorSpaceString::try_from_json(&mut ctx, "#/token", &json!(input));
            assert!(
                matches!(state, ParseState::Parsed(ref v) if v == &expected),
                "failed for input: {input}"
            );
            assert!(ctx.errors.is_empty(), "unexpected error for input: {input}");
        }
    }

    #[test]
    fn color_space_string_rejects_unknown_string() {
        let mut ctx = test_ctx();

        let state = ColorSpaceString::try_from_json(&mut ctx, "#/token", &json!("cmyk"));

        assert!(matches!(state, ParseState::Failed(_)));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyValue);
        assert_eq!(ctx.errors[0].path, "#/token");
    }

    #[test]
    fn color_space_string_rejects_non_string() {
        let mut ctx = test_ctx();

        let state = ColorSpaceString::try_from_json(&mut ctx, "#/token", &json!(42));

        assert!(matches!(state, ParseState::Failed(_)));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyValue);
        assert_eq!(ctx.errors[0].path, "#/token");
    }

    // ── ColorAlpha ────────────────────────────────────────────────────────

    #[test]
    fn color_alpha_parses_number() {
        let mut ctx = test_ctx();

        let state = ColorAlpha::try_from_json(&mut ctx, "#/token", &json!(0.5));

        assert!(matches!(
            state,
            ParseState::Parsed(ColorAlpha(JsonNumber(_)))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn color_alpha_rejects_non_number() {
        let mut ctx = test_ctx();

        let state = ColorAlpha::try_from_json(&mut ctx, "#/token", &json!("0.5"));

        assert!(matches!(state, ParseState::Failed(_)));
    }

    // ── ColorHexString ────────────────────────────────────────────────────

    #[test]
    fn color_hex_string_parses_hex_string() {
        let mut ctx = test_ctx();

        let state = ColorHexString::try_from_json(&mut ctx, "#/token", &json!("#ff0000"));

        assert!(matches!(state, ParseState::Parsed(ColorHexString(ref s)) if s == "#ff0000"));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn color_hex_string_rejects_non_string() {
        let mut ctx = test_ctx();

        let state = ColorHexString::try_from_json(&mut ctx, "#/token", &json!(255));

        assert!(matches!(state, ParseState::Failed(_)));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyValue);
        assert_eq!(ctx.errors[0].path, "#/token");
    }

    // ── ColorTokenValue ───────────────────────────────────────────────────

    #[test]
    fn color_token_value_parses_minimal_valid_object() {
        let mut ctx = test_ctx();
        let input = json!({
            "colorSpace": "srgb",
            "components": [1, 0, 0]
        });

        let state = ColorTokenValue::try_from_json(&mut ctx, "#/token", &input);

        let parsed = match state {
            ParseState::Parsed(v) => v,
            _ => panic!("expected parsed color token value"),
        };

        assert!(matches!(
            parsed.color_space,
            RefOrLiteral::Literal(ColorSpaceString::SRGB)
        ));
        assert!(matches!(parsed.components, RefOrLiteral::Literal(_)));
        assert!(parsed.alpha.is_none());
        assert!(parsed.hex.is_none());
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn color_token_value_parses_with_optional_alpha_and_hex() {
        let mut ctx = test_ctx();
        let input = json!({
            "colorSpace": "hsl",
            "components": [210, "none", 0.5],
            "alpha": 0.8,
            "hex": "#3399ff"
        });

        let state = ColorTokenValue::try_from_json(&mut ctx, "#/token", &input);

        let parsed = match state {
            ParseState::Parsed(v) => v,
            _ => panic!("expected parsed color token value"),
        };

        assert!(matches!(
            parsed.color_space,
            RefOrLiteral::Literal(ColorSpaceString::HSL)
        ));
        assert!(parsed.alpha.is_some());
        assert!(parsed.hex.is_some());
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn color_token_value_parses_with_refs_for_color_space_and_components() {
        let mut ctx = test_ctx();
        let input = json!({
            "colorSpace": { "$ref": "#/colors/space" },
            "components": { "$ref": "#/colors/components" }
        });

        let state = ColorTokenValue::try_from_json(&mut ctx, "#/token", &input);

        let parsed = match state {
            ParseState::Parsed(v) => v,
            _ => panic!("expected parsed color token value"),
        };

        assert!(matches!(parsed.color_space, RefOrLiteral::Ref(_)));
        assert!(matches!(parsed.components, RefOrLiteral::Ref(_)));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn color_token_value_rejects_non_object() {
        let mut ctx = test_ctx();

        let state = ColorTokenValue::try_from_json(&mut ctx, "#/token", &json!("red"));

        assert!(matches!(state, ParseState::Failed(_)));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyValue);
        assert_eq!(ctx.errors[0].path, "#/token");
    }

    #[test]
    fn color_token_value_reports_missing_color_space() {
        let mut ctx = test_ctx();
        let input = json!({ "components": [1, 0, 0] });

        let state = ColorTokenValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::Failed(_)));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[0].path, "#/token/colorSpace");
    }

    #[test]
    fn color_token_value_reports_missing_components() {
        let mut ctx = test_ctx();
        let input = json!({ "colorSpace": "srgb" });

        let state = ColorTokenValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::Failed(_)));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[0].path, "#/token/components");
    }

    #[test]
    fn color_token_value_reports_invalid_color_space_value() {
        let mut ctx = test_ctx();
        let input = json!({ "colorSpace": "neon", "components": [1, 0, 0] });

        let state = ColorTokenValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::Failed(_)));
        assert!(ctx.errors.iter().any(|e| {
            e.code == DiagnosticCode::InvalidPropertyValue && e.path == "#/token/colorSpace"
        }));
    }

    #[test]
    fn color_token_value_reports_invalid_components_length() {
        let mut ctx = test_ctx();
        let input = json!({ "colorSpace": "srgb", "components": [1, 0] });

        let state = ColorTokenValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::Failed(_)));
        assert!(
            ctx.errors
                .iter()
                .any(|e| { e.code == DiagnosticCode::InvalidPropertyValue })
        );
    }

    #[test]
    fn color_token_value_reports_invalid_components_type() {
        let mut ctx = test_ctx();
        let input = json!({ "colorSpace": "srgb", "components": 123 });

        let state = ColorTokenValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::Failed(_)));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn color_token_value_reports_invalid_optional_alpha_type() {
        let mut ctx = test_ctx();
        let input = json!({
            "colorSpace": "srgb",
            "components": [1, 0, 0],
            "alpha": "0.5"
        });

        let state = ColorTokenValue::try_from_json(&mut ctx, "#/token", &input);

        let parsed = match state {
            ParseState::Parsed(v) => v,
            _ => panic!("expected parsed color token value"),
        };

        assert!(parsed.alpha.is_none());
        assert!(ctx.errors.iter().any(|e| {
            e.code == DiagnosticCode::InvalidPropertyValue && e.path == "#/token/alpha"
        }));
    }
}
