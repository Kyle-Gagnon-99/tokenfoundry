//! The `duration` module defines the data structures for duration tokens defined in the DTCG specification,
//! which represents a duration in the UI, such as the duration of an animation or transition.

use crate::ir::{JsonNumber, RefOr, TryFromJson, require_enum_string_with_mapping, require_object};

/// The DTCG specification only accepts the "value" property for duration tokens, which is a string,
/// to be either "ms" for milliseconds or "s" for seconds. This enum represents the unit of the duration token value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DurationUnit {
    Ms,
    S,
}

impl<'a> TryFromJson<'a> for DurationUnit {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        require_enum_string_with_mapping(
            ctx,
            path,
            "unit",
            value,
            |s| match s {
                "ms" => Some(Self::Ms),
                "s" => Some(Self::S),
                _ => None,
            },
            "ms, s",
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DurationValue(JsonNumber);

impl<'a> TryFromJson<'a> for DurationValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        JsonNumber::try_from_json(ctx, path, value).map(DurationValue)
    }
}

/// Represents a duration token value, which consists of a numeric value and a unit
#[derive(Debug, Clone, PartialEq)]
pub struct DurationTokenValue {
    /// The numeric value of the duration, which can be either a signed integer or a float
    pub value: RefOr<DurationValue>,
    /// The unit of the duration, which can be either milliseconds (ms) or seconds (s)
    pub unit: RefOr<DurationUnit>,
}

impl<'a> TryFromJson<'a> for DurationTokenValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        let obj = require_object(ctx, path, value, "duration token")?;

        let value = obj.required_field::<RefOr<DurationValue>>(ctx, path, "value");
        let unit = obj.required_field::<RefOr<DurationUnit>>(ctx, path, "unit");

        match (value, unit) {
            (Some(value), Some(unit)) => Some(DurationTokenValue { value, unit }),
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
    fn parses_supported_duration_units() {
        let mut ctx = parser_context();

        let ms = DurationUnit::try_from_json(&mut ctx, "/token/unit", &json!("ms"));
        let s = DurationUnit::try_from_json(&mut ctx, "/token/unit", &json!("s"));

        assert_eq!(ms, Some(DurationUnit::Ms));
        assert_eq!(s, Some(DurationUnit::S));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn rejects_unknown_duration_unit() {
        let mut ctx = parser_context();

        let unit = DurationUnit::try_from_json(&mut ctx, "/token/unit", &json!("sec"));

        assert_eq!(unit, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidEnumValue);
        assert_eq!(ctx.errors[0].path, "/token/unit");
        assert!(ctx.errors[0].message.contains("ms, s"));
    }

    #[test]
    fn parses_duration_token_with_literal_fields() {
        let mut ctx = parser_context();
        let value = json!({
            "value": 200,
            "unit": "ms"
        });

        let parsed = DurationTokenValue::try_from_json(&mut ctx, "/token", &value);

        assert_eq!(
            parsed,
            Some(DurationTokenValue {
                value: RefOr::Literal(DurationValue(JsonNumber(Number::from(200)))),
                unit: RefOr::Literal(DurationUnit::Ms),
            })
        );
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn parses_duration_token_with_references() {
        let mut ctx = parser_context();
        let value = json!({
            "value": { "$ref": "#/base/animation/value" },
            "unit": { "$ref": "#/base/animation/unit" }
        });

        let parsed = DurationTokenValue::try_from_json(&mut ctx, "/token", &value).unwrap();

        match parsed.value {
            RefOr::Ref(json_ref) => assert_eq!(
                json_ref,
                JsonRef::new_local_pointer(
                    "#/base/animation/value".to_string(),
                    JsonPointer::from("#/base/animation/value")
                )
            ),
            RefOr::Literal(_) => panic!("expected value field to parse as a reference"),
        }
        match parsed.unit {
            RefOr::Ref(json_ref) => assert_eq!(
                json_ref,
                JsonRef::new_local_pointer(
                    "#/base/animation/unit".to_string(),
                    JsonPointer::from("#/base/animation/unit")
                )
            ),
            RefOr::Literal(_) => panic!("expected unit field to parse as a reference"),
        }
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn reports_missing_required_unit_field() {
        let mut ctx = parser_context();
        let value = json!({
            "value": 250
        });

        let parsed = DurationTokenValue::try_from_json(&mut ctx, "/token", &value);

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[0].path, "/token/unit");
    }

    #[test]
    fn reports_missing_required_value_field() {
        let mut ctx = parser_context();
        let value = json!({
            "unit": "ms"
        });

        let parsed = DurationTokenValue::try_from_json(&mut ctx, "/token", &value);

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[0].path, "/token/value");
    }

    #[test]
    fn reports_both_missing_required_fields() {
        let mut ctx = parser_context();
        let value = json!({});

        let parsed = DurationTokenValue::try_from_json(&mut ctx, "/token", &value);

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 2);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[0].path, "/token/value");
        assert_eq!(ctx.errors[1].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[1].path, "/token/unit");
    }

    #[test]
    fn parses_negative_fractional_duration_value() {
        let mut ctx = parser_context();
        let value = json!({
            "value": -0.5,
            "unit": "s"
        });

        let parsed = DurationTokenValue::try_from_json(&mut ctx, "/token", &value);

        assert_eq!(
            parsed,
            Some(DurationTokenValue {
                value: RefOr::Literal(DurationValue(JsonNumber(Number::from_f64(-0.5).unwrap()))),
                unit: RefOr::Literal(DurationUnit::S),
            })
        );
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn reports_invalid_top_level_shape() {
        let mut ctx = parser_context();

        let parsed = DurationTokenValue::try_from_json(&mut ctx, "/token", &json!(250));

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyType);
        assert_eq!(ctx.errors[0].path, "/token");
    }

    #[test]
    fn reports_invalid_unit_type() {
        let mut ctx = parser_context();
        let value = json!({
            "value": 250,
            "unit": 2
        });

        let parsed = DurationTokenValue::try_from_json(&mut ctx, "/token", &value);

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
            "unit": "ms"
        });

        let parsed = DurationTokenValue::try_from_json(&mut ctx, "/token", &value);

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidReference);
        assert_eq!(ctx.errors[0].path, "/token/value");
    }

    #[test]
    fn reports_invalid_value_and_unit_fields() {
        let mut ctx = parser_context();
        let value = json!({
            "value": "fast",
            "unit": "minutes"
        });

        let parsed = DurationTokenValue::try_from_json(&mut ctx, "/token", &value);

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 2);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyType);
        assert_eq!(ctx.errors[0].path, "/token/value");
        assert_eq!(ctx.errors[1].code, DiagnosticCode::InvalidEnumValue);
        assert_eq!(ctx.errors[1].path, "/token/unit");
    }
}
