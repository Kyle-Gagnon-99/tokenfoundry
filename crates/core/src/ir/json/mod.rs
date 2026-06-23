//! The `json` module provides IR data structures for JSON values

use crate::{
    errors::{DiagnosticCode, ParseDiagnosticCode},
    ir::JsonPointer,
    parsing::ParserContext,
};

pub mod utils;

use serde_json::Value;
pub use utils::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosticEmission {
    /// A diagnostic was already emitted for this error, so no additional diagnostics should be emitted.
    Emitted,
    /// No diagnostic has been emitted for this error, so the caller is responsible for emitting a diagnostic.
    Silent,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseFailure {
    pub kind: ParseFailureKind,
    pub diagnostic: DiagnosticEmission,
}

impl ParseFailure {
    pub fn silent(kind: ParseFailureKind) -> Self {
        Self {
            kind,
            diagnostic: DiagnosticEmission::Silent,
        }
    }

    pub fn emitted(kind: ParseFailureKind) -> Self {
        Self {
            kind,
            diagnostic: DiagnosticEmission::Emitted,
        }
    }

    pub fn mark_emitted(&mut self) -> Self {
        self.diagnostic = DiagnosticEmission::Emitted;
        self.clone()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JsonValueKind {
    String,
    Number,
    Object,
    Array,
    Bool,
    Null,
}

impl From<Value> for JsonValueKind {
    fn from(value: Value) -> Self {
        match value {
            Value::String(_) => JsonValueKind::String,
            Value::Number(_) => JsonValueKind::Number,
            Value::Object(_) => JsonValueKind::Object,
            Value::Array(_) => JsonValueKind::Array,
            Value::Bool(_) => JsonValueKind::Bool,
            Value::Null => JsonValueKind::Null,
        }
    }
}

impl From<&Value> for JsonValueKind {
    fn from(value: &Value) -> Self {
        match value {
            Value::String(_) => JsonValueKind::String,
            Value::Number(_) => JsonValueKind::Number,
            Value::Object(_) => JsonValueKind::Object,
            Value::Array(_) => JsonValueKind::Array,
            Value::Bool(_) => JsonValueKind::Bool,
            Value::Null => JsonValueKind::Null,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseFailureKind {
    WrongType {
        expected: JsonValueKind,
        actual: JsonValueKind,
    },
    MissingRequiredField {
        field: String,
    },
    MissingRequiredFields {
        fields: Vec<String>,
    },
    InvalidValue,
    InvalidReference,
}

/// Represents the result of trying to parse a JSON value into a specific type. This is used as the return type for the `try_from_json` method of the
/// `TryFromJson` trait. If the value was successfully parsed into the expected type, `Parsed` should be returned.
/// If the value was not in the expected format but is still valid JSON, `NoMatch` should be returned (indicating that this parser did not match the value, but other parsers may still be able to parse it).
/// If the value was not in the expected format and is not valid for this type, `Failed` should be returned, and any relevant diagnostic errors should have already been pushed to the `ParserContext`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParseState<T> {
    /// The value was successfully parsed into the expected type, and the parsed value is included.
    Parsed(T),
    /// The parser does not match, but the value is still valid JSON, so let other parsers try to parse this value.
    NoMatch,
    /// The value was not in the expected format and is not valid for this type, so the parsing failed
    Failed(ParseFailure),
}

impl<T> ParseState<T> {
    pub fn failed_silent(kind: ParseFailureKind) -> Self {
        Self::Failed(ParseFailure::silent(kind))
    }

    pub fn failed_emitted(kind: ParseFailureKind) -> Self {
        Self::Failed(ParseFailure::emitted(kind))
    }
}

/// Allows a struct to be converted / created from a JSON value, with error handling through the `ParserContext`.
pub trait TryFromJson<'a>: Sized {
    /// Tries to create an instance of `Self` from a JSON value. If the JSON value is not in the expected format, any diagnostic errors should be pushed
    /// to the `ParserContext`, and `ParseState::Failed` should be returned.
    ///
    /// The `path` argument is a JSON pointer string that indicates the location of the value being parsed within the overall JSON structure.
    /// This is useful for error reporting, as it allows the parser to indicate exactly where in the input JSON the error occurred.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The parser context to which any diagnostic errors should be pushed, if found.
    /// * `path` - A JSON pointer string indicating the location of the value being parsed within the overall JSON structure.
    /// * `value` - The JSON value to be parsed into an instance of `Self`
    ///
    /// # Returns
    ///
    /// ParseState<Self> if the JSON was successfully parsed into an instance of `Self`, or `ParseState::Failed` if the JSON was not in the expected format
    /// or was invalid for this type. Any relevant diagnostic errors should have already been pushed to the `ParserContext`.
    fn try_from_json(
        ctx: &mut ParserContext,
        path: &JsonPointer,
        value: &'a serde_json::Value,
    ) -> ParseState<Self>;
}

/// A helper enum to represent a JSON number than can be any range from i64::MIN to u64::MAX, as well as f64 values.
/// This is useful for parsing numeric values from JSON where the number could be anything, and we need to support
/// the full range of JSON numbers without losing precision or causing overflow issues.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct JsonNumber(pub serde_json::Number);

impl JsonNumber {
    pub fn from_value(value: &serde_json::Value) -> Option<Self> {
        match value {
            serde_json::Value::Number(num) => Some(Self(num.clone())),
            _ => None,
        }
    }

    pub fn new(num: serde_json::Number) -> Self {
        Self(num)
    }
}

impl<'a> TryFromJson<'a> for JsonNumber {
    fn try_from_json(
        _ctx: &mut ParserContext,
        _path: &JsonPointer,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match value {
            serde_json::Value::Number(num) => ParseState::Parsed(JsonNumber(num.clone())),
            _ => ParseState::failed_silent(ParseFailureKind::WrongType {
                expected: JsonValueKind::Number,
                actual: JsonValueKind::from(value),
            }),
        }
    }
}

/// A helper struct to represent a JSON object, which is just a wrapper around `serde_json::Map<String, Value>`.
/// Should not be used directly, but useful for utility functions that need to work with JSON objects
/// more specifically, the `serde_json::Map<String, Value>` with custom error handling.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JsonObject<'a>(pub &'a serde_json::Map<String, serde_json::Value>);

impl<'a> JsonObject<'a> {
    fn find_field<'b>(&self, names: &'b [&str]) -> Option<(&'b str, &'a serde_json::Value)> {
        for name in names {
            if let Some(value) = self.get(name) {
                return Some((name, value));
            }
        }
        None
    }

    /// Tries to create a `JsonObject` from a JSON value. If the value is not an object, an error will be pushed to the `ParserContext`.
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
    pub fn from_value(value: &'a serde_json::Value) -> Option<Self> {
        match value {
            serde_json::Value::Object(map) => Some(Self(map)),
            _ => None,
        }
    }

    /// Creates a `JsonObject` from a `serde_json::Map<String, Value>`. This is just a simple wrapper around the map, and does not do any error handling.
    /// This is useful for utility functions that have already verified that a JSON value is an object, and just want to work with it as a `JsonObject`
    /// without having to convert back and forth between `serde_json::Value` and `serde_json::Map<String, Value>`.
    ///
    /// # Arguments
    ///
    /// * `map` - The `serde_json::Map<String, Value>` to be wrapped as a `JsonObject`
    ///
    /// # Returns
    ///
    /// A `JsonObject` that wraps the given map.
    pub fn new(map: &'a serde_json::Map<String, serde_json::Value>) -> Self {
        Self(map)
    }

    /// Gets a reference to the JSON value associated with the given key in the object. Returns `None` if the key is not present.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to look up in the JSON object
    ///
    /// # Returns
    ///
    /// An `Option<&'a serde_json::Value>` which is `Some(&Value)` if the key is present in the object, or `None` if the key is not present.
    pub fn get(&self, key: &str) -> Option<&'a serde_json::Value> {
        self.0.get(key)
    }

    /// Checks if the JSON object contains the given key.
    ///
    /// # Arguments
    ///
    /// * `key` - The key to check for in the JSON object
    ///
    /// # Returns
    ///
    /// `true` if the key is present in the JSON object, or `false` if the key is not present.
    pub fn contains_key(&self, key: &str) -> bool {
        self.0.contains_key(key)
    }

    /// Tries to parse a required field from the JSON object using the `TryFromJson` trait.
    /// If the field is missing, an error is pushed to the `ParserContext`. If the field is present but cannot be parsed into the expected type,
    /// any errors should have already been pushed to the `ParserContext` by the `TryFromJson` implementation, and `None` will be returned.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The parser context to which any diagnostic errors should be pushed, if found.
    /// * `path` - A JSON pointer string indicating the location of the value being parsed within the overall JSON structure.
    /// * `field_name` - The name of the field to parse from the JSON object
    ///
    /// * `T` - The expected type of the field, which must implement the `TryFromJson` trait.
    ///
    /// # Returns
    ///
    /// An `Option<T>` which is `Some(parsed_value)` if the field was successfully parsed into the expected type, or `None` if the field
    /// was missing or could not be parsed (in which case any errors should have already been pushed to the `ParserContext`).
    pub fn required_field<T: TryFromJson<'a>>(
        &self,
        ctx: &mut ParserContext,
        path: &mut JsonPointer,
        field_name: &str,
    ) -> Option<T> {
        let field_path = path.with_segment(field_name);

        let value = match self.get(field_name) {
            Some(v) => v,
            None => {
                ctx.diagnostics()
                    .error(DiagnosticCode::Parse(
                        ParseDiagnosticCode::MissingRequiredField {
                            field: field_name.to_string(),
                        },
                    ))
                    .json_pointer(field_path.clone())
                    .emit();
                return None;
            }
        };

        match T::try_from_json(ctx, &field_path, value) {
            ParseState::Parsed(v) => Some(v),
            ParseState::Failed(inv) => {
                // The caller may have already emitted an error for this invalid field, so only emit an error if the ownership is `Silent`.
                // If the ownership is `Emitted`, that means the caller has already emitted an error for this invalid field, so we should not
                // emit another error to avoid duplicate errors for the same issue.
                if inv.ownership == DiagnosticEmission::Silent {
                    ctx.push_to_errors(
                        DiagnosticCode::InvalidPropertyType,
                        format!("Invalid property type for field '{}'", field_name),
                        format!("{}/{}", path, field_name),
                    );
                }
                return None;
            }
            ParseState::NoMatch => None,
        }
    }

    /// Same as `required_field`, but supports multiple accepted JSON key names.
    /// The first entry is treated as canonical for diagnostics.
    pub fn required_field_any<T: TryFromJson<'a>>(
        &self,
        ctx: &mut ParserContext,
        path: &str,
        field_names: &[&str],
    ) -> Option<T> {
        if field_names.is_empty() {
            return None;
        }

        let canonical_name = field_names[0];
        let field_path = format!("{}/{}", path, canonical_name);

        let (actual_name, value) = match self.find_field(field_names) {
            Some(found) => found,
            None => {
                ctx.push_to_errors(
                    DiagnosticCode::MissingRequiredProperty,
                    format!("Missing required field '{}' at {}", canonical_name, path),
                    field_path,
                );
                return None;
            }
        };

        let actual_field_path = format!("{}/{}", path, actual_name);
        match T::try_from_json(ctx, &actual_field_path, value) {
            ParseState::Parsed(v) => Some(v),
            ParseState::Failed(inv) => {
                if inv.ownership == DiagnosticEmission::Silent {
                    ctx.push_to_errors(
                        DiagnosticCode::InvalidPropertyType,
                        format!("Invalid property type for field '{}'", canonical_name),
                        actual_field_path,
                    );
                }
                None
            }
            ParseState::NoMatch => None,
        }
    }

    /// Tries to parse an optional field from the JSON object using the `TryFromJson` trait.
    ///
    /// If the field is missing, `None` will be returned without pushing an error to the `ParserContext`.
    /// If the field is present but cannot be parsed into the expected type, any errors should have already been pushed to the `ParserContext`
    /// by the `TryFromJson` implementation, and `None` will be returned.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The parser context to which any diagnostic errors should be pushed, if found.
    /// * `path` - A JSON pointer string indicating the location of the value being parsed within the overall JSON structure.
    /// * `field_name` - The name of the field to parse from the JSON object
    ///
    /// * `T` - The expected type of the field, which must implement the `TryFromJson` trait.
    ///
    /// # Returns
    ///
    /// An `Option<T>` which is `Some(parsed_value)` if the field was successfully parsed into the expected type, or `None` if the field was missing or could not be parsed (in which
    /// case any errors should have already been pushed to the `ParserContext`).
    pub fn optional_field<T: TryFromJson<'a>>(
        &self,
        ctx: &mut ParserContext,
        path: &str,
        field_name: &str,
    ) -> Option<T> {
        let field_path = format!("{}/{}", path, field_name);

        let value = match self.get(field_name) {
            Some(v) => v,
            None => {
                return None;
            }
        };

        match T::try_from_json(ctx, &field_path, value) {
            ParseState::Parsed(v) => Some(v),
            ParseState::Failed(inv) => {
                // The caller may have already emitted an error for this invalid field, so only emit an error if the ownership is `Silent`.
                // If the ownership is `Emitted`, that means the caller has already emitted an error for this invalid field, so we should not
                // emit another error to avoid duplicate errors for the same issue.
                if inv.ownership == DiagnosticEmission::Silent {
                    ctx.push_to_errors(
                        DiagnosticCode::InvalidPropertyType,
                        format!("Invalid property type for field '{}'", field_name),
                        format!("{}/{}", path, field_name),
                    );
                }
                return None;
            }
            ParseState::NoMatch => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidPropertyType,
                    format!(
                        "Invalid property type for field '{}' at {}",
                        field_name, path
                    ),
                    field_path,
                );
                None
            }
        }
    }

    /// Same as `optional_field`, but supports multiple accepted JSON key names.
    /// The first entry is treated as canonical for diagnostics.
    pub fn optional_field_any<T: TryFromJson<'a>>(
        &self,
        ctx: &mut ParserContext,
        path: &str,
        field_names: &[&str],
    ) -> Option<T> {
        if field_names.is_empty() {
            return None;
        }

        let canonical_name = field_names[0];
        let (actual_name, value) = match self.find_field(field_names) {
            Some(found) => found,
            None => return None,
        };

        let actual_field_path = format!("{}/{}", path, actual_name);
        match T::try_from_json(ctx, &actual_field_path, value) {
            ParseState::Parsed(v) => Some(v),
            ParseState::Failed(inv) => {
                if inv.ownership == DiagnosticEmission::Silent {
                    ctx.push_to_errors(
                        DiagnosticCode::InvalidPropertyType,
                        format!("Invalid property type for field '{}'", canonical_name),
                        actual_field_path,
                    );
                }
                None
            }
            ParseState::NoMatch => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidPropertyType,
                    format!(
                        "Invalid property type for field '{}' at {}",
                        canonical_name, path
                    ),
                    format!("{}/{}", path, canonical_name),
                );
                None
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonArray<'a>(pub &'a Vec<serde_json::Value>);

impl<'a> JsonArray<'a> {
    pub fn from_value(value: &'a serde_json::Value) -> Option<Self> {
        match value {
            serde_json::Value::Array(arr) => Some(Self(arr)),
            _ => None,
        }
    }

    pub fn new(arr: &'a Vec<serde_json::Value>) -> Self {
        Self(arr)
    }

    pub fn get(&self, index: usize) -> Option<&'a serde_json::Value> {
        self.0.get(index)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn parse_for_each<T: TryFromJson<'a>>(
        &self,
        ctx: &mut ParserContext,
        path: &str,
    ) -> Option<Vec<T>> {
        let mut results = Vec::new();
        for (index, value) in self.0.iter().enumerate() {
            let item_path = format!("{}/{}", path, index);
            match T::try_from_json(ctx, &item_path, value) {
                ParseState::Parsed(v) => results.push(v),
                _ => {
                    ctx.push_to_errors(
                        DiagnosticCode::InvalidPropertyType,
                        format!("Invalid element at {}", item_path),
                        item_path,
                    );
                    return None;
                }
            }
        }
        Some(results)
    }
}

impl<'a> TryFromJson<'a> for String {
    fn try_from_json(
        _ctx: &mut ParserContext,
        _path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match value {
            serde_json::Value::String(s) => ParseState::Parsed(s.to_owned()),
            _ => ParseState::NoMatch,
        }
    }
}

impl<'a> TryFromJson<'a> for bool {
    fn try_from_json(
        _ctx: &mut ParserContext,
        _path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match value {
            serde_json::Value::Bool(b) => ParseState::Parsed(*b),
            _ => ParseState::NoMatch,
        }
    }
}
