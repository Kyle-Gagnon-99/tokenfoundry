//! The `utils` module provides utility functions and types for parsing tokens from raw JSON values to IR values
//! These utilities are used across different token types and can help with common parsing logic.
use crate::{
    ParserContext,
    errors::DiagnosticCode,
    token::{
        DeprecationValue, TryFromJson,
        ir::{JsonPointer, JsonRef, JsonRefKind, RefOr},
    },
};

/// The `JsonFloatOrInteger` enum represents a JSON value that can be either a float or a signed integer.
pub enum JsonFloatOrInteger {
    Integer(i64),
    Float(f64),
}

/// Parses a deprecation value from a JSON value.
///
/// Converts a `serde_json::Value` into a `DeprecationValue` enum variant.
/// Supports two formats:
/// - Boolean values are converted to `DeprecationValue::Boolean`
/// - String values are converted to `DeprecationValue::WithMessage`
///
/// # Arguments
///
/// * `value` - A reference to a `serde_json::Value` to parse
///
/// # Returns
///
/// Returns `Some(DeprecationValue)` if the value is a boolean or string,
/// otherwise returns `None` if the value is of an unsupported type.
pub fn parse_deprecation_value(value: &serde_json::Value) -> Option<DeprecationValue> {
    match value {
        serde_json::Value::Bool(b) => Some(DeprecationValue::Boolean(*b)),
        serde_json::Value::String(s) => Some(DeprecationValue::WithMessage(s.clone())),
        _ => None,
    }
}

/// Utility function to require a field from a JSON object and return it as a reference.
///
/// If the field is missing, it pushes an error to the parser context and returns `None`.
///
/// # Arguments
/// * `ctx` - The parser context to push errors to
/// * `path` - The path to the current token, used for error reporting
/// * `map` - The JSON object to extract the field from
/// * `field_name` - The name of the required field to extract
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
///
/// * `ctx` - The parser context to push errors to
/// * `path` - The path to the current token, used for error reporting
/// * `value` - The JSON value to check if it's a number
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
/// * `ctx` - The parser context to push errors to
/// * `path` - The path to the current token, used for error reporting
/// * `value` - The JSON value to check if it's a string
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
/// * `ctx` - The parser context to push errors to
/// * `path` - The path to the current token, used for error reporting
/// * `value` - The JSON value to check if it's an array
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
/// * `ctx` - The parser context to push errors to
/// * `path` - The path to the current token, used for error reporting
/// * `value` - The JSON value to check if it's an object
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
/// * `ctx` - The parser context to push errors to
/// * `path` - The path to the current token, used for error reporting
/// * `value` - The JSON value to check if it's a string that matches one of the valid enum values
/// * `valid_values` - A slice of valid string values that the input value should match
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
/// * `ctx` - The parser context to push errors to
/// * `path` - The path to the current token, used for error reporting
/// * `field_name` - The name of the field being parsed, used for error reporting
/// * `value` - The JSON value to check if it's a string that matches one of the valid enum values
/// * `parse` - A function that takes a string and returns an `Option<T>`, where `T` is the internal representation of the enum value. This function should return `Some(T)` if the input string matches a valid enum value, or `None` if it does not.
/// * `expected_values` - A string representation of the expected valid values, used for error reporting in case of an invalid value. This can be a comma-separated list of valid values or any other format suitable for error messages.
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
/// * `ctx` - The parser context to push errors to
/// * `path` - The path to the current token, used for error reporting
/// * `value` - The JSON value to check if it's a number that can be either a float or an integer
///
/// # Returns
///
/// Returns `Some(JsonFloatOrInteger)` if the value is a number that can be either a float or an integer, otherwise returns `None` and pushes an error to the context.
pub fn require_float_or_integer<'a>(
    ctx: &mut ParserContext,
    path: &'a str,
    value: &'a serde_json::Value,
) -> Option<JsonFloatOrInteger> {
    match value {
        serde_json::Value::Number(n) => {
            if n.is_i64() {
                Some(JsonFloatOrInteger::Integer(n.as_i64().unwrap()))
            } else if n.is_f64() {
                Some(JsonFloatOrInteger::Float(n.as_f64().unwrap()))
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

pub fn parse_ref_or_type<'a, T>(
    ctx: &mut ParserContext,
    path: &'a str,
    value: &'a serde_json::Value,
    parse: impl FnOnce(&mut ParserContext, &str, &'a serde_json::Value) -> Option<T>,
) -> Option<RefOr<T>>
where
    T: TryFromJson<'a>,
{
    /* match value {
        serde_json::Value::Object(map) => {
            // Check to see if the object has a $ref property and no other properties
            // We may have a $ref, but if there are other properties, this is not a valid reference
            // The specification does not explicitly state that a reference object cannot have other properties, but we
            // will consider it invalid to avoid ambiguity and potential errors in parsing.
            let has_ref = map.contains_key("$ref");

            if has_ref && map.len() == 1 {
                // This is a reference object
                let raw_ref = require_object_field(ctx, path, map, "$ref")?;
                let ref_str = require_string(ctx, path, raw_ref)?;


            }
        }
        _ => parse(ctx, path, value),
    } */
    Some(RefOr::Literal(parse(ctx, path, value)?))
}
