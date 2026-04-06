//! The `dimension` module defines the data structures and parsing logic for dimension tokens, which represents
//! a single dimension in the UI, such as a position, width, height, radius, size, or thickness
use crate::{
    errors::DiagnosticCode,
    token::{
        ParseState, TryFromJson,
        utils::{
            JsonFloatOrInteger, require_enum_string_with_mapping, require_float_or_integer,
            require_object_field,
        },
    },
};

pub enum DimensionNumber {
    Integer(i64),
    Float(f64),
}

pub enum DimensionUnit {
    Px,
    Rem,
}

/// Represents a dimension token value, which consists of a numeric value and a unit
pub struct DimensionTokenValue {
    /// The numeric value of the dimension, which can be either a signed integer or a float
    pub value: DimensionNumber,
    /// The unit of the dimension, which can be either pixels (px) or rems (rem)
    pub unit: DimensionUnit,
}

impl<'a> TryFromJson<'a> for DimensionTokenValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self>
    where
        Self: Sized,
    {
        match value {
            serde_json::Value::Object(map) => {
                // The DTCG specification defines that a dimension token value shall have a "value" property, which is a number
                // and a "unit" property, which is a string that can be either "px" or "rem"
                let raw_value = require_object_field(ctx, path, map, "value");
                let raw_unit = require_object_field(ctx, path, map, "unit");

                let parsed_value = raw_value.and_then(|raw| {
                    require_float_or_integer(ctx, path, raw).map(|number| match number {
                        JsonFloatOrInteger::Integer(value) => DimensionNumber::Integer(value),
                        JsonFloatOrInteger::Float(value) => DimensionNumber::Float(value),
                    })
                });

                let parsed_unit = raw_unit.and_then(|raw| {
                    require_enum_string_with_mapping(
                        ctx,
                        path,
                        "unit",
                        raw,
                        |s| match s {
                            "px" => Some(DimensionUnit::Px),
                            "rem" => Some(DimensionUnit::Rem),
                            _ => None,
                        },
                        "px, rem",
                    )
                });

                match (parsed_value, parsed_unit) {
                    (Some(value), Some(unit)) => ParseState::Parsed(Self { value, unit }),
                    _ => ParseState::Skipped,
                }
            }
            _ => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidPropertyType,
                    format!("The dimension token must be an object"),
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

    fn parse_dimension(
        value: &serde_json::Value,
    ) -> (ParseState<DimensionTokenValue>, ParserContext) {
        let mut ctx = make_context();
        let path = String::from("tokens.size.small");

        let result = DimensionTokenValue::try_from_json(&mut ctx, &path, value);

        (result, ctx)
    }

    #[test]
    fn parses_integer_value_with_px_unit() {
        let value = json!({
            "value": 16,
            "unit": "px"
        });

        let (result, ctx) = parse_dimension(&value);

        assert!(ctx.errors.is_empty());

        let ParseState::Parsed(parsed) = result else {
            panic!("expected dimension token to parse successfully");
        };

        assert!(matches!(parsed.value, DimensionNumber::Integer(16)));
        assert!(matches!(parsed.unit, DimensionUnit::Px));
    }

    #[test]
    fn parses_float_value_with_rem_unit() {
        let value = json!({
            "value": 1.5,
            "unit": "rem"
        });

        let (result, ctx) = parse_dimension(&value);

        assert!(ctx.errors.is_empty());

        let ParseState::Parsed(parsed) = result else {
            panic!("expected dimension token to parse successfully");
        };

        match parsed.value {
            DimensionNumber::Float(number) => assert_eq!(number, 1.5),
            DimensionNumber::Integer(_) => panic!("expected a float dimension value"),
        }
        assert!(matches!(parsed.unit, DimensionUnit::Rem));
    }

    #[test]
    fn parses_negative_integer_value() {
        let value = json!({
            "value": -24,
            "unit": "px"
        });

        let (result, ctx) = parse_dimension(&value);

        assert!(ctx.errors.is_empty());

        let ParseState::Parsed(parsed) = result else {
            panic!("expected dimension token to parse successfully");
        };

        assert!(matches!(parsed.value, DimensionNumber::Integer(-24)));
        assert!(matches!(parsed.unit, DimensionUnit::Px));
    }

    #[test]
    fn skips_when_required_unit_is_missing() {
        let value = json!({
            "value": 8
        });

        let (result, ctx) = parse_dimension(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[0].path, "tokens.size.small");
    }

    #[test]
    fn skips_when_both_required_fields_are_missing() {
        let value = json!({});

        let (result, ctx) = parse_dimension(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 2);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[0].message, "Missing required field: value");
        assert_eq!(ctx.errors[1].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[1].message, "Missing required field: unit");
    }

    #[test]
    fn skips_when_value_or_unit_is_invalid_and_collects_both_diagnostics() {
        let value = json!({
            "value": "12",
            "unit": "em"
        });

        let (result, ctx) = parse_dimension(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 2);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyType);
        assert_eq!(ctx.errors[1].code, DiagnosticCode::InvalidEnumValue);
        assert!(
            ctx.errors
                .iter()
                .all(|error| error.path == "tokens.size.small")
        );
    }

    #[test]
    fn skips_when_unit_has_wrong_json_type() {
        let value = json!({
            "value": 12,
            "unit": true
        });

        let (result, ctx) = parse_dimension(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyType);
        assert_eq!(ctx.errors[0].message, "Expected a string, but found: true");
    }

    #[test]
    fn returns_fatal_error_for_single_json_value() {
        let value = json!("16px");

        let (result, ctx) = parse_dimension(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert!(ctx.errors.len() == 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyType);
        assert_eq!(
            ctx.errors[0].message,
            "The dimension token must be an object"
        );
    }
}
