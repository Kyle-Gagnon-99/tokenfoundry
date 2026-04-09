//! The `duration` module defines the data structures for duration tokens defined in the DTCG specification,
//! which represents a duration in the UI, such as the duration of an animation or transition.

use crate::{
    errors::DiagnosticCode,
    ir::RefOr,
    token::{
        ParseState, TryFromJson, TryFromJsonField,
        utils::{
            FieldPresence, FloatOrInteger, parse_field, require_enum_string_with_mapping,
            require_float_or_integer,
        },
    },
};

/// The DTCG specification only accepts the "value" property for duration tokens, which is a string,
/// to be either "ms" for milliseconds or "s" for seconds. This enum represents the unit of the duration token value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum DurationUnit {
    Ms,
    S,
}

impl<'a> TryFromJsonField<'a> for DurationUnit {
    fn try_from_json_field(
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
pub struct DurationValue(FloatOrInteger);

impl<'a> TryFromJsonField<'a> for DurationValue {
    fn try_from_json_field(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        require_float_or_integer(ctx, path, value).map(DurationValue)
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
    ) -> ParseState<Self> {
        match value {
            serde_json::Value::Object(map) => {
                let value =
                    parse_field::<DurationValue>(ctx, path, map, "value", FieldPresence::Required);
                let unit =
                    parse_field::<DurationUnit>(ctx, path, map, "unit", FieldPresence::Required);

                match (value, unit) {
                    (Some(value), Some(unit)) => {
                        ParseState::Parsed(DurationTokenValue { value, unit })
                    }
                    _ => ParseState::Skipped, // The errors have already been pushed by parse_field, so we just return Skipped here
                }
            }
            _ => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidPropertyType,
                    format!("The duration token value should be an object with 'value' and 'unit' properties, but found {}", value),
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

    fn parse_duration(
        value: &serde_json::Value,
    ) -> (ParseState<DurationTokenValue>, ParserContext) {
        let mut ctx = make_context();
        let result =
            DurationTokenValue::try_from_json(&mut ctx, "tokens.motion.fast.duration", value);
        (result, ctx)
    }

    #[test]
    fn parses_integer_duration_with_ms_unit() {
        let value = json!({
            "value": 150,
            "unit": "ms"
        });

        let (result, ctx) = parse_duration(&value);

        assert!(ctx.errors.is_empty());

        let ParseState::Parsed(parsed) = result else {
            panic!("expected duration token to parse successfully");
        };

        assert_eq!(
            parsed.value,
            RefOr::Literal(DurationValue(FloatOrInteger::Integer(150)))
        );
        assert!(matches!(parsed.unit, RefOr::Literal(DurationUnit::Ms)));
    }

    #[test]
    fn parses_float_duration_with_seconds_unit() {
        let value = json!({
            "value": 0.25,
            "unit": "s"
        });

        let (result, ctx) = parse_duration(&value);

        assert!(ctx.errors.is_empty());

        let ParseState::Parsed(parsed) = result else {
            panic!("expected duration token to parse successfully");
        };

        assert_eq!(
            parsed.value,
            RefOr::Literal(DurationValue(FloatOrInteger::Float(0.25)))
        );
        assert!(matches!(parsed.unit, RefOr::Literal(DurationUnit::S)));
    }

    #[test]
    fn parses_negative_duration_value() {
        let value = json!({
            "value": -50,
            "unit": "ms"
        });

        let (result, ctx) = parse_duration(&value);

        assert!(ctx.errors.is_empty());

        let ParseState::Parsed(parsed) = result else {
            panic!("expected duration token to parse successfully");
        };

        assert_eq!(
            parsed.value,
            RefOr::Literal(DurationValue(FloatOrInteger::Integer(-50)))
        );
        assert!(matches!(parsed.unit, RefOr::Literal(DurationUnit::Ms)));
    }

    #[test]
    fn skips_when_required_unit_is_missing() {
        let value = json!({
            "value": 100
        });

        let (result, ctx) = parse_duration(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[0].message, "Missing required field: unit");
        assert_eq!(ctx.errors[0].path, "tokens.motion.fast.duration");
    }

    #[test]
    fn skips_when_both_required_fields_are_missing() {
        let value = json!({});

        let (result, ctx) = parse_duration(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 2);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[0].message, "Missing required field: value");
        assert_eq!(ctx.errors[1].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[1].message, "Missing required field: unit");
    }

    #[test]
    fn skips_when_value_and_unit_are_invalid() {
        let value = json!({
            "value": "100",
            "unit": "minutes"
        });

        let (result, ctx) = parse_duration(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 2);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyType);
        assert_eq!(
            ctx.errors[0].message,
            "Expected a number, but found: \"100\""
        );
        assert_eq!(ctx.errors[1].code, DiagnosticCode::InvalidEnumValue);
        assert_eq!(
            ctx.errors[1].message,
            "Expected one of ms, s for the field 'unit', but got 'minutes'"
        );
    }

    #[test]
    fn skips_when_unit_has_wrong_json_type() {
        let value = json!({
            "value": 100,
            "unit": true
        });

        let (result, ctx) = parse_duration(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyType);
        assert_eq!(ctx.errors[0].message, "Expected a string, but found: true");
    }

    #[test]
    fn skips_when_value_is_not_an_object() {
        let value = json!("150ms");

        let (result, ctx) = parse_duration(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyType);
        assert_eq!(
            ctx.errors[0].message,
            "The duration token value should be an object with 'value' and 'unit' properties, but found \"150ms\""
        );
        assert_eq!(ctx.errors[0].path, "tokens.motion.fast.duration");
    }
}
