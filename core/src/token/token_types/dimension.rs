//! The `dimension` module defines the data structures and parsing logic for dimension tokens, which represents
//! a single dimension in the UI, such as a position, width, height, radius, size, or thickness
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

#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub struct DimensionNumber(FloatOrInteger);

impl<'a> TryFromJsonField<'a> for DimensionNumber {
    fn try_from_json_field(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        require_float_or_integer(ctx, path, value).map(DimensionNumber)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum DimensionUnit {
    Px,
    Rem,
}

impl<'a> TryFromJsonField<'a> for DimensionUnit {
    fn try_from_json_field(
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
    pub value: RefOr<DimensionNumber>,
    /// The unit of the dimension, which can be either pixels (px) or rems (rem)
    pub unit: RefOr<DimensionUnit>,
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
                let value_field = parse_field::<DimensionNumber>(
                    ctx,
                    path,
                    map,
                    "value",
                    FieldPresence::Required,
                );
                let unit_field =
                    parse_field::<DimensionUnit>(ctx, path, map, "unit", FieldPresence::Required);

                match (value_field, unit_field) {
                    (Some(value), Some(unit)) => {
                        ParseState::Parsed(DimensionTokenValue { value, unit })
                    }
                    _ => ParseState::Skipped,
                }
            }
            _ => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidPropertyType,
                    "The dimension token must be an object",
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
    use crate::{
        FileFormat, ParserContext,
        errors::DiagnosticCode,
        ir::{JsonPointer, JsonRef},
    };
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

    fn expect_parsed(result: ParseState<DimensionTokenValue>) -> DimensionTokenValue {
        let ParseState::Parsed(parsed) = result else {
            panic!("expected dimension token to parse successfully");
        };

        parsed
    }

    #[test]
    fn parses_integer_value_with_px_unit() {
        let value = json!({
            "value": 16,
            "unit": "px"
        });

        let (result, ctx) = parse_dimension(&value);

        assert!(ctx.errors.is_empty());

        let parsed = expect_parsed(result);

        assert_eq!(
            parsed.value,
            RefOr::Literal(DimensionNumber(FloatOrInteger::Integer(16)))
        );
        assert_eq!(parsed.unit, RefOr::Literal(DimensionUnit::Px));
    }

    #[test]
    fn parses_float_value_with_rem_unit() {
        let value = json!({
            "value": 1.5,
            "unit": "rem"
        });

        let (result, ctx) = parse_dimension(&value);

        assert!(ctx.errors.is_empty());

        let parsed = expect_parsed(result);

        assert_eq!(
            parsed.value,
            RefOr::Literal(DimensionNumber(FloatOrInteger::Float(1.5)))
        );
        assert_eq!(parsed.unit, RefOr::Literal(DimensionUnit::Rem));
    }

    #[test]
    fn parses_negative_integer_value() {
        let value = json!({
            "value": -24,
            "unit": "px"
        });

        let (result, ctx) = parse_dimension(&value);

        assert!(ctx.errors.is_empty());

        let parsed = expect_parsed(result);

        assert_eq!(
            parsed.value,
            RefOr::Literal(DimensionNumber(FloatOrInteger::Integer(-24)))
        );
        assert_eq!(parsed.unit, RefOr::Literal(DimensionUnit::Px));
    }

    #[test]
    fn parses_ref_value_with_literal_unit() {
        let value = json!({
            "value": { "$ref": "#/tokens/size/base/value" },
            "unit": "px"
        });

        let (result, ctx) = parse_dimension(&value);

        assert!(ctx.errors.is_empty());

        let parsed = expect_parsed(result);

        assert_eq!(
            parsed.value,
            RefOr::Ref(JsonRef::new_local_pointer(
                "#/tokens/size/base/value".to_string(),
                JsonPointer::from("#/tokens/size/base/value"),
            ))
        );
        assert_eq!(parsed.unit, RefOr::Literal(DimensionUnit::Px));
    }

    #[test]
    fn parses_literal_value_with_ref_unit() {
        let value = json!({
            "value": 16,
            "unit": { "$ref": "#/tokens/size/base/unit" }
        });

        let (result, ctx) = parse_dimension(&value);

        assert!(ctx.errors.is_empty());

        let parsed = expect_parsed(result);

        assert_eq!(
            parsed.value,
            RefOr::Literal(DimensionNumber(FloatOrInteger::Integer(16)))
        );
        assert_eq!(
            parsed.unit,
            RefOr::Ref(JsonRef::new_local_pointer(
                "#/tokens/size/base/unit".to_string(),
                JsonPointer::from("#/tokens/size/base/unit"),
            ))
        );
    }

    #[test]
    fn parses_empty_string_refs_for_value_and_unit() {
        let value = json!({
            "value": { "$ref": "" },
            "unit": { "$ref": "" }
        });

        let (result, ctx) = parse_dimension(&value);

        assert!(ctx.errors.is_empty());

        let parsed = expect_parsed(result);

        assert_eq!(
            parsed.value,
            RefOr::Ref(JsonRef::new_local_pointer(
                String::new(),
                JsonPointer::new(),
            ))
        );
        assert_eq!(
            parsed.unit,
            RefOr::Ref(JsonRef::new_local_pointer(
                String::new(),
                JsonPointer::new(),
            ))
        );
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
    fn skips_when_value_ref_pointer_is_invalid() {
        let value = json!({
            "value": { "$ref": "tokens/size/base/value" },
            "unit": "px"
        });

        let (result, ctx) = parse_dimension(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidReference);
        assert_eq!(
            ctx.errors[0].message,
            "Invalid JSON pointer: tokens/size/base/value"
        );
        assert_eq!(ctx.errors[0].path, "tokens.size.small");
    }

    #[test]
    fn skips_when_ref_object_has_extra_properties() {
        let value = json!({
            "value": {
                "$ref": "#/tokens/size/base/value",
                "fallback": 16
            },
            "unit": "px"
        });

        let (result, ctx) = parse_dimension(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyType);
        assert_eq!(
            ctx.errors[0].message,
            "Expected a number, but found: {\"$ref\":\"#/tokens/size/base/value\",\"fallback\":16}"
        );
        assert_eq!(ctx.errors[0].path, "tokens.size.small");
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
