//! The `utils` module provides utility functions and types for parsing tokens from raw JSON values to IR values
//! These utilities are used across different token types and can help with common parsing logic.
use crate::{
    ParserContext,
    errors::DiagnosticCode,
    ir::{RefOr, parse_ref_or_value},
    token::TryFromJsonField,
};

/// The `FloatOrInteger` enum represents a JSON value that can be either a float or a signed integer.
#[derive(Debug, Clone, PartialEq)]
pub enum FloatOrInteger {
    Integer(i64),
    Float(f64),
}

impl TryFrom<&serde_json::Number> for FloatOrInteger {
    type Error = ();

    fn try_from(value: &serde_json::Number) -> Result<Self, Self::Error> {
        if value.is_i64() {
            Ok(FloatOrInteger::Integer(value.as_i64().unwrap()))
        } else if value.is_f64() {
            Ok(FloatOrInteger::Float(value.as_f64().unwrap()))
        } else {
            Err(())
        }
    }
}

impl<'a> TryFromJsonField<'a> for FloatOrInteger {
    fn try_from_json_field(
        ctx: &mut ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        match value {
            serde_json::Value::Number(num) => Self::try_from(num).ok(),
            _ => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidPropertyType,
                    format!("Expected a number, but found: {value}"),
                    path.into(),
                );
                None
            }
        }
    }
}

/// Utility function to require a field from a JSON object and return it as a reference.
///
/// If the field is missing, it pushes an error to the parser context and returns `None`.
///
/// # Arguments
/// - `ctx` - The parser context to push errors to
/// - `path` - The path to the current token, used for error reporting
/// - `map` - The JSON object to extract the field from
/// - `field_name` - The name of the required field to extract
///
/// # Returns
///
/// Returns `Some(&serde_json::Value)` if the field is present, otherwise returns `None` and pushes an error to the context.
pub fn require_object_field<'a>(
    ctx: &mut ParserContext,
    path: &'a str,
    map: &'a serde_json::Map<String, serde_json::Value>,
    field_name: &str,
) -> Option<&'a serde_json::Value> {
    match map.get(field_name) {
        Some(value) => Some(value),
        None => {
            ctx.push_to_errors(
                DiagnosticCode::MissingRequiredProperty,
                format!("Missing required field: {}", field_name),
                path.into(),
            );
            None
        }
    }
}

/// Utility function to require a number as the JSON value
///
/// If the value is not a number, it pushes an error to the parser context and returns `None`.
///
/// # Arguments
/// - `ctx` - The parser context to push errors to
/// - `path` - The path to the current token, used for error reporting
/// - `value` - The JSON value to check if it's a number
///
/// # Returns
///
/// Returns `Some(&serde_json::Number)` if the value is a number, otherwise returns `None` and pushes an error to the context.
pub fn require_number<'a>(
    ctx: &mut ParserContext,
    path: &'a str,
    value: &'a serde_json::Value,
) -> Option<&'a serde_json::Number> {
    match value {
        serde_json::Value::Number(n) => Some(n),
        other => {
            ctx.push_to_errors(
                DiagnosticCode::InvalidPropertyType,
                format!("Expected a number, but found: {other}"),
                path.into(),
            );
            None
        }
    }
}

/// Utility function to require a string as the JSON value
///
/// If the value is not a string, it pushes an error to the parser context and returns `None`.
///
/// # Arguments
///
/// - `ctx` - The parser context to push errors to
/// - `path` - The path to the current token, used for error reporting
/// - `value` - The JSON value to check if it's a string
///
/// # Returns
///
/// Returns `Some(&String)` if the value is a string, otherwise returns `None` and pushes an error to the context.
pub fn require_string<'a>(
    ctx: &mut ParserContext,
    path: &'a str,
    value: &'a serde_json::Value,
) -> Option<&'a str> {
    match value {
        serde_json::Value::String(s) => Some(s),
        other => {
            ctx.push_to_errors(
                DiagnosticCode::InvalidPropertyType,
                format!("Expected a string, but found: {other}"),
                path.into(),
            );
            None
        }
    }
}

/// Utility function to require an array as the JSON value
///
/// If the value is not an array, it pushes an error to the parser context and returns `None`.
///
/// # Arguments
///
/// - `ctx` - The parser context to push errors to
/// - `path` - The path to the current token, used for error reporting
/// - `value` - The JSON value to check if it's an array
///
/// # Returns
///
/// Returns `Some(&Vec<serde_json::Value>)` if the value is an array, otherwise returns `None` and pushes an error to the context.
pub fn require_array<'a>(
    ctx: &mut ParserContext,
    path: &'a str,
    value: &'a serde_json::Value,
) -> Option<&'a Vec<serde_json::Value>> {
    match value {
        serde_json::Value::Array(arr) => Some(arr),
        other => {
            ctx.push_to_errors(
                DiagnosticCode::InvalidPropertyType,
                format!("Expected an array, but found: {other}"),
                path.into(),
            );
            None
        }
    }
}

/// Utility function to require an object as the JSON value
///
/// If the value is not an object, it pushes an error to the parser context and returns `None`.
///
/// # Arguments
///
/// - `ctx` - The parser context to push errors to
/// - `path` - The path to the current token, used for error reporting
/// - `value` - The JSON value to check if it's an object
///
/// # Returns
///
/// Returns `Some(&serde_json::Map<String, serde_json::Value>)` if the value is an object, otherwise returns `None` and pushes an error to the context.
pub fn require_object<'a>(
    ctx: &mut ParserContext,
    path: &'a str,
    value: &'a serde_json::Value,
) -> Option<&'a serde_json::Map<String, serde_json::Value>> {
    match value {
        serde_json::Value::Object(obj) => Some(obj),
        other => {
            ctx.push_to_errors(
                DiagnosticCode::InvalidPropertyType,
                format!("Expected an object, but found: {other}"),
                path.into(),
            );
            None
        }
    }
}

/// Utility function to require a string that matches one of the valid enum values
///
/// If the value is not a string or does not match any of the valid enum values, it pushes an error to the parser context and returns `None`.
///
/// # Arguments
///
/// - `ctx` - The parser context to push errors to
/// - `path` - The path to the current token, used for error reporting
/// - `value` - The JSON value to check if it's a string that matches one of the valid enum values
/// - `valid_values` - A slice of valid string values that the input value should match
///
/// # Returns
///
/// Returns `Some(&String)` if the value is a string that matches one of the valid enum values, otherwise returns `None` and pushes an error to the context.
pub fn require_enum_string<'a>(
    ctx: &mut ParserContext,
    path: &'a str,
    value: &'a serde_json::Value,
    valid_values: &[&str],
) -> Option<&'a String> {
    match value {
        serde_json::Value::String(s) => {
            if valid_values.contains(&s.as_str()) {
                Some(s)
            } else {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidEnumValue,
                    format!(
                        "Expected one of the following values: {:?}, but found: {s}",
                        valid_values
                    ),
                    path.into(),
                );
                None
            }
        }
        other => {
            ctx.push_to_errors(
                DiagnosticCode::InvalidPropertyType,
                format!("Expected a string, but found: {other}"),
                path.into(),
            );
            None
        }
    }
}

/// Utility function to require a string that matches one of the valid enum values and returns the corresponding mapped value
///
/// If the value is not a string or does not match any of the valid enum values, it pushes an error to the parser context and returns `None`.
/// This function is useful when the valid enum values in the JSON are different from the internal representation in Rust,
/// allowing for a mapping between the two.
///
/// # Arguments
///
/// - `ctx` - The parser context to push errors to
/// - `path` - The path to the current token, used for error reporting
/// - `field_name` - The name of the field being parsed, used for error reporting
/// - `value` - The JSON value to check if it's a string that matches one of the valid enum values
/// - `parse` - A function that takes a string and returns an `Option<T>`, where `T` is the internal representation of the enum value. This function should return `Some(T)` if the input string matches a valid enum value, or `None` if it does not.
/// - `expected_values` - A string representation of the expected valid values, used for error reporting in case of an invalid value. This can be a comma-separated list of valid values or any other format suitable for error messages.
///
/// # Returns
///
/// Returns `Some(&T)` if the value is a string that matches one of the valid enum values, returning the corresponding mapped value, otherwise returns `None` and pushes an error to the context.
pub fn require_enum_string_with_mapping<'a, T>(
    ctx: &mut ParserContext,
    path: &'a str,
    field_name: &str,
    value: &'a serde_json::Value,
    parse: impl FnOnce(&'a str) -> Option<T>,
    expected_values: &str,
) -> Option<T> {
    let string_value = require_string(ctx, path, value)?;

    match parse(string_value) {
        Some(parsed) => Some(parsed),
        None => {
            ctx.push_to_errors(
                DiagnosticCode::InvalidEnumValue,
                format!("Expected one of {expected_values} for the field '{field_name}', but got '{string_value}'"),
                path.into(),
            );
            None
        }
    }
}

/// Utility function to require a number that can be either a float or an integer
///
/// If the value is not a number, it pushes an error to the parser context and returns `None`.
///
/// # Arguments
///
/// - `ctx` - The parser context to push errors to
/// - `path` - The path to the current token, used for error reporting
/// - `value` - The JSON value to check if it's a number that can be either a float or an integer
///
/// # Returns
///
/// Returns `Some(JsonFloatOrInteger)` if the value is a number that can be either a float or an integer, otherwise returns `None` and pushes an error to the context.
pub fn require_float_or_integer<'a>(
    ctx: &mut ParserContext,
    path: &'a str,
    value: &'a serde_json::Value,
) -> Option<FloatOrInteger> {
    match value {
        serde_json::Value::Number(n) => {
            if n.is_i64() {
                Some(FloatOrInteger::Integer(n.as_i64().unwrap()))
            } else if n.is_f64() {
                Some(FloatOrInteger::Float(n.as_f64().unwrap()))
            } else {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidPropertyType,
                    format!("Expected a number, but found: {n}"),
                    path.into(),
                );
                None
            }
        }
        other => {
            ctx.push_to_errors(
                DiagnosticCode::InvalidPropertyType,
                format!("Expected a number, but found: {other}"),
                path.into(),
            );
            None
        }
    }
}

pub enum FieldPresence {
    Required,
    Optional,
}

pub fn parse_field<'a, T: TryFromJsonField<'a>>(
    ctx: &mut ParserContext,
    path: &str,
    value: &'a serde_json::Map<String, serde_json::Value>,
    field_name: &str,
    presence: FieldPresence,
) -> Option<RefOr<T>> {
    // First, get if the field is present
    let raw_value_opt = value.get(field_name);

    // If the field is required but not present, push an error and return None
    // Otherwise, if the field is optional and not present, we can just return None without an error
    let raw_value = match (raw_value_opt, presence) {
        (Some(raw), _) => raw,
        (None, FieldPresence::Required) => {
            ctx.push_to_errors(
                DiagnosticCode::MissingRequiredProperty,
                format!("Missing required field: {}", field_name),
                path.into(),
            );
            return None;
        }
        (None, FieldPresence::Optional) => return None,
    };

    parse_ref_or_value(ctx, path, raw_value)
}

pub fn parse_field_no_ref<'a, T: TryFromJsonField<'a>>(
    ctx: &mut ParserContext,
    path: &str,
    value: &'a serde_json::Map<String, serde_json::Value>,
    field_name: &str,
    presence: FieldPresence,
) -> Option<T> {
    // First, get if the field is present
    let raw_value_opt = value.get(field_name);

    // If the field is required but not present, push an error and return None
    // Otherwise, if the field is optional and not present, we can just return None without an error
    let raw_value = match (raw_value_opt, presence) {
        (Some(raw), _) => raw,
        (None, FieldPresence::Required) => {
            ctx.push_to_errors(
                DiagnosticCode::MissingRequiredProperty,
                format!("Missing required field: {}", field_name),
                path.into(),
            );
            return None;
        }
        (None, FieldPresence::Optional) => return None,
    };

    T::try_from_json_field(ctx, path, raw_value)
}

//-----------------------------------------
// Implement TryFromJsonField for common types like String, JsonFloatOrInteger, etc. to allow them to be used with parse_field
//-----------------------------------------

impl<'a> TryFromJsonField<'a> for String {
    fn try_from_json_field(
        ctx: &mut ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        require_string(ctx, path, value).map(|s| s.to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        FileFormat, ParserContext,
        errors::DiagnosticCode,
        ir::{JsonPointer, JsonRef, RefOr},
        token::TryFromJsonField,
    };
    use serde_json::json;

    #[derive(Debug, Clone, PartialEq, Eq, Hash)]
    struct TestField {
        value: String,
    }

    impl<'a> TryFromJsonField<'a> for TestField {
        fn try_from_json_field(
            ctx: &mut ParserContext,
            path: &str,
            value: &'a serde_json::Value,
        ) -> Option<Self> {
            let object = require_object(ctx, path, value)?;
            let raw_value = require_object_field(ctx, path, object, "value")?;
            let value = require_string(ctx, path, raw_value)?;

            Some(Self {
                value: value.to_owned(),
            })
        }
    }

    fn make_context() -> ParserContext {
        ParserContext::new(String::from("test.json"), FileFormat::Json, String::new())
    }

    fn parse_test_field(
        container: &serde_json::Value,
        field_name: &str,
        presence: FieldPresence,
    ) -> (Option<RefOr<TestField>>, ParserContext) {
        let mut ctx = make_context();
        let path = "tokens.motion.fast";
        let map = container.as_object().expect("expected container object");
        let result = parse_field::<TestField>(&mut ctx, path, map, field_name, presence);
        (result, ctx)
    }

    #[test]
    fn returns_none_without_error_for_missing_optional_field() {
        let container = json!({});

        let (result, ctx) = parse_test_field(&container, "value", FieldPresence::Optional);

        assert!(result.is_none());
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn returns_none_with_error_for_missing_required_field() {
        let container = json!({});

        let (result, ctx) = parse_test_field(&container, "value", FieldPresence::Required);

        assert!(result.is_none());
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[0].message, "Missing required field: value");
        assert_eq!(ctx.errors[0].path, "tokens.motion.fast");
    }

    #[test]
    fn parses_literal_field_value() {
        let container = json!({
            "value": {
                "value": "fast"
            }
        });

        let (result, ctx) = parse_test_field(&container, "value", FieldPresence::Required);

        assert!(ctx.errors.is_empty());
        assert_eq!(
            result,
            Some(RefOr::Literal(TestField {
                value: String::from("fast"),
            }))
        );
    }

    #[test]
    fn parses_empty_string_ref_field_value() {
        let container = json!({
            "value": {
                "$ref": ""
            }
        });

        let (result, ctx) = parse_test_field(&container, "value", FieldPresence::Required);

        assert!(ctx.errors.is_empty());
        assert_eq!(
            result,
            Some(RefOr::Ref(JsonRef::new_local_pointer(
                String::new(),
                JsonPointer::new(),
            )))
        );
    }

    #[test]
    fn parses_json_pointer_ref_field_value() {
        let container = json!({
            "value": {
                "$ref": "#/tokens/motion/fast"
            }
        });

        let (result, ctx) = parse_test_field(&container, "value", FieldPresence::Required);

        assert!(ctx.errors.is_empty());
        assert_eq!(
            result,
            Some(RefOr::Ref(JsonRef::new_local_pointer(
                String::from("#/tokens/motion/fast"),
                JsonPointer::from("#/tokens/motion/fast"),
            )))
        );
    }

    #[test]
    fn treats_object_with_ref_and_other_properties_as_literal_value() {
        let container = json!({
            "value": {
                "$ref": "#/tokens/motion/slow",
                "value": "fast"
            }
        });

        let (result, ctx) = parse_test_field(&container, "value", FieldPresence::Required);

        assert!(ctx.errors.is_empty());
        assert_eq!(
            result,
            Some(RefOr::Literal(TestField {
                value: String::from("fast"),
            }))
        );
    }

    #[test]
    fn returns_error_for_invalid_ref_pointer() {
        let container = json!({
            "value": {
                "$ref": "tokens/motion/fast"
            }
        });

        let (result, ctx) = parse_test_field(&container, "value", FieldPresence::Required);

        assert!(result.is_none());
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidReference);
        assert_eq!(
            ctx.errors[0].message,
            "Invalid JSON pointer: tokens/motion/fast"
        );
        assert_eq!(ctx.errors[0].path, "tokens.motion.fast");
    }

    #[test]
    fn propagates_literal_parser_errors() {
        let container = json!({
            "value": 42
        });

        let (result, ctx) = parse_test_field(&container, "value", FieldPresence::Required);

        assert!(result.is_none());
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyType);
        assert_eq!(ctx.errors[0].message, "Expected an object, but found: 42");
        assert_eq!(ctx.errors[0].path, "tokens.motion.fast");
    }
}
