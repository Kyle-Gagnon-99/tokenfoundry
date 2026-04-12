//! The `dimension` module defines the data structures and parsing logic for dimension tokens, which represents
//! a single dimension in the UI, such as a position, width, height, radius, size, or thickness
use crate::ir::{JsonNumber, RefOr, TryFromJson, require_enum_string_with_mapping, require_object};

#[derive(Debug, Clone, PartialEq)]
pub struct DimensionValue(JsonNumber);

impl<'a> TryFromJson<'a> for DimensionValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        JsonNumber::try_from_json(ctx, path, value).map(DimensionValue)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DimensionUnit {
    Px,
    Rem,
}

impl<'a> TryFromJson<'a> for DimensionUnit {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        require_enum_string_with_mapping(
            ctx,
            path,
            "dimension unit",
            value,
            |s| match s {
                "px" => Some(DimensionUnit::Px),
                "rem" => Some(DimensionUnit::Rem),
                _ => None,
            },
            "px, rem",
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
/// Represents a dimension token value, which consists of a numeric value and a unit
pub struct DimensionTokenValue {
    /// The numeric value of the dimension, which can be either a signed integer or a float
    pub value: RefOr<DimensionValue>,
    /// The unit of the dimension, which can be either pixels (px) or rems (rem)
    pub unit: RefOr<DimensionUnit>,
}

impl<'a> TryFromJson<'a> for DimensionTokenValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self>
    where
        Self: Sized,
    {
        let obj = require_object(ctx, path, value, "dimension token")?;
        let value = obj.required_field::<RefOr<DimensionValue>>(ctx, path, "value");
        let unit = obj.required_field::<RefOr<DimensionUnit>>(ctx, path, "unit");

        match (value, unit) {
            (Some(value), Some(unit)) => Some(DimensionTokenValue { value, unit }),
            _ => None, // The errors have already been pushed by required_field, so we just return None here
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        FileFormat, ParserContext,
        errors::DiagnosticCode,
        ir::{JsonNumber, JsonPointer, JsonRef, RefOr, TryFromJson},
    };
    use serde_json::{Number, json};

    fn parser_context() -> ParserContext {
        ParserContext::new("tests.json".into(), FileFormat::Json, String::new())
    }

    #[test]
    fn parses_supported_dimension_units() {
        let mut ctx = parser_context();

        let px = DimensionUnit::try_from_json(&mut ctx, "/token/unit", &json!("px"));
        let rem = DimensionUnit::try_from_json(&mut ctx, "/token/unit", &json!("rem"));

        assert_eq!(px, Some(DimensionUnit::Px));
        assert_eq!(rem, Some(DimensionUnit::Rem));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn rejects_unknown_dimension_unit() {
        let mut ctx = parser_context();

        let unit = DimensionUnit::try_from_json(&mut ctx, "/token/unit", &json!("em"));

        assert_eq!(unit, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidEnumValue);
        assert_eq!(ctx.errors[0].path, "/token/unit");
        assert!(ctx.errors[0].message.contains("px, rem"));
    }

    #[test]
    fn parses_dimension_token_with_literal_fields() {
        let mut ctx = parser_context();
        let value = json!({
            "value": 16,
            "unit": "px"
        });

        let parsed = DimensionTokenValue::try_from_json(&mut ctx, "/token", &value);

        assert_eq!(
            parsed,
            Some(DimensionTokenValue {
                value: RefOr::Literal(DimensionValue(JsonNumber(Number::from(16)))),
                unit: RefOr::Literal(DimensionUnit::Px),
            })
        );
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn parses_dimension_token_with_references() {
        let mut ctx = parser_context();
        let value = json!({
            "value": { "$ref": "#/base/spacing/value" },
            "unit": { "$ref": "#/base/spacing/unit" }
        });

        let parsed = DimensionTokenValue::try_from_json(&mut ctx, "/token", &value).unwrap();

        match parsed.value {
            RefOr::Ref(json_ref) => assert_eq!(
                json_ref,
                JsonRef::new_local_pointer(
                    "#/base/spacing/value".to_string(),
                    JsonPointer::from("#/base/spacing/value")
                ),
            ),
            RefOr::Literal(_) => panic!("expected value field to parse as a reference"),
        }
        match parsed.unit {
            RefOr::Ref(json_ref) => assert_eq!(
                json_ref,
                JsonRef::new_local_pointer(
                    "#/base/spacing/unit".to_string(),
                    JsonPointer::from("#/base/spacing/unit")
                ),
            ),
            RefOr::Literal(_) => panic!("expected unit field to parse as a reference"),
        }
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn reports_missing_required_unit_field() {
        let mut ctx = parser_context();
        let value = json!({
            "value": 12
        });

        let parsed = DimensionTokenValue::try_from_json(&mut ctx, "/token", &value);

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[0].path, "/token/unit");
    }

    #[test]
    fn reports_missing_required_value_field() {
        let mut ctx = parser_context();
        let value = json!({
            "unit": "px"
        });

        let parsed = DimensionTokenValue::try_from_json(&mut ctx, "/token", &value);

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[0].path, "/token/value");
    }

    #[test]
    fn reports_both_missing_required_fields() {
        let mut ctx = parser_context();
        let value = json!({});

        let parsed = DimensionTokenValue::try_from_json(&mut ctx, "/token", &value);

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 2);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[0].path, "/token/value");
        assert_eq!(ctx.errors[1].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[1].path, "/token/unit");
    }

    #[test]
    fn parses_negative_fractional_dimension_value() {
        let mut ctx = parser_context();
        let value = json!({
            "value": -1.25,
            "unit": "rem"
        });

        let parsed = DimensionTokenValue::try_from_json(&mut ctx, "/token", &value);

        assert_eq!(
            parsed,
            Some(DimensionTokenValue {
                value: RefOr::Literal(DimensionValue(JsonNumber(Number::from_f64(-1.25).unwrap()))),
                unit: RefOr::Literal(DimensionUnit::Rem),
            })
        );
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn reports_invalid_top_level_shape() {
        let mut ctx = parser_context();

        let parsed = DimensionTokenValue::try_from_json(&mut ctx, "/token", &json!(12));

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyType);
        assert_eq!(ctx.errors[0].path, "/token");
    }

    #[test]
    fn reports_invalid_unit_type() {
        let mut ctx = parser_context();
        let value = json!({
            "value": 12,
            "unit": 4
        });

        let parsed = DimensionTokenValue::try_from_json(&mut ctx, "/token", &value);

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyType);
        assert_eq!(ctx.errors[0].path, "/token/unit");
    }

    #[test]
    fn reports_invalid_reference_value_field() {
        let mut ctx = parser_context();
        let value = json!({
            "value": { "$ref": "not-a-pointer" },
            "unit": "px"
        });

        let parsed = DimensionTokenValue::try_from_json(&mut ctx, "/token", &value);

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidReference);
        assert_eq!(ctx.errors[0].path, "/token/value");
    }

    #[test]
    fn reports_invalid_value_and_unit_fields() {
        let mut ctx = parser_context();
        let value = json!({
            "value": "large",
            "unit": "em"
        });

        let parsed = DimensionTokenValue::try_from_json(&mut ctx, "/token", &value);

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 2);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyType);
        assert_eq!(ctx.errors[0].path, "/token/value");
        assert_eq!(ctx.errors[1].code, DiagnosticCode::InvalidEnumValue);
        assert_eq!(ctx.errors[1].path, "/token/unit");
    }
}
