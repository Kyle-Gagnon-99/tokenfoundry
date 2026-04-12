//! The `utils` module in `json` contains utility functions for parsing JSON values into IR data structures, with error handling

use crate::{ParserContext, errors::DiagnosticCode, ir::JsonObject};

/// Utility function to require a JSON value to be an object, and return it as a `JsonObject` if it is,
/// or push an error to the `ParserContext` if it is not.
///
/// # Arguments
///
/// * `ctx` - The parser context to which any diagnostic errors should be pushed, if found.
/// * `path` - A JSON pointer string indicating the location of the value being parsed within the overall JSON structure.
/// * `value` - The JSON value to be parsed into a `JsonObject`
///
/// # Returns
///
/// An `Option<JsonObject>` which is `Some(JsonObject)` if the JSON value was successfully parsed into a `JsonObject`,
/// or `None` if the JSON value was not an object.
pub fn require_object<'a>(
    ctx: &mut ParserContext,
    path: &str,
    value: &'a serde_json::Value,
    obj_name: &str,
) -> Option<JsonObject<'a>> {
    JsonObject::from_value(value).or_else(|| {
        ctx.push_to_errors(
            DiagnosticCode::InvalidPropertyType,
            format!("Expected object for {}, but found: {}", obj_name, value),
            path.into(),
        );
        None
    })
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
