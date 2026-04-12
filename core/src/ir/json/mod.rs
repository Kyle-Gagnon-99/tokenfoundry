//! The `json` module provides IR data structures for JSON values

use crate::{
    ParserContext,
    errors::DiagnosticCode,
    ir::{JsonPointer, JsonRef},
};

pub mod utils;

pub use utils::*;

/// Allows a struct to be converted / created from a JSON value, with error handling through the `ParserContext`.
pub trait TryFromJson<'a>: Sized {
    /// Tries to create an instance of `Self` from a JSON value. If the JSON value is not in the expected format, any diagnostic errors should be pushed
    /// to the `ParserContext`, and `None` should be returned.
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
    /// An `Option<Self>` which is `Some(instance)` if the JSON value was successfully parsed into an instance of `Self`,
    /// or `None` if the JSON value was not able to be parsed (in which case any errors should have already been pushed to the `ParserContext`).
    fn try_from_json(
        ctx: &mut ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self>;
}

/// A helper enum to represent a JSON number than can be any range from i64::MIN to u64::MAX, as well as f64 values.
/// This is useful for parsing numeric values from JSON where the number could be anything, and we need to support
/// the full range of JSON numbers without losing precision or causing overflow issues.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct JsonNumber(pub serde_json::Number);

impl JsonNumber {
    pub fn from_value(value: &serde_json::Value) -> Option<Self> {
        match value {
            serde_json::Value::Number(num) => Some(Self(num.clone())),
            _ => None,
        }
    }
}

impl<'a> TryFromJson<'a> for JsonNumber {
    fn try_from_json(
        ctx: &mut ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        match value {
            serde_json::Value::Number(num) => Some(JsonNumber(num.clone())),
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

/// A helper struct to represent a JSON object, which is just a wrapper around `serde_json::Map<String, Value>`.
/// Should not be used directly, but useful for utility functions that need to work with JSON objects
/// more specifically, the `serde_json::Map<String, Value>` with custom error handling.
pub struct JsonObject<'a>(pub &'a serde_json::Map<String, serde_json::Value>);

impl<'a> JsonObject<'a> {
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
    /// An `Option<T>` which is `Some(parsed_value)` if the field was successfully parsed into the expected type, or `None` if the
    /// field was missing or could not be parsed (in which case any errors should have already been pushed to the `ParserContext`).
    pub fn required_field<T: TryFromJson<'a>>(
        &self,
        ctx: &mut ParserContext,
        path: &str,
        field_name: &str,
    ) -> Option<T> {
        let raw_value = match self.get(field_name) {
            Some(value) => value,
            None => {
                ctx.push_to_errors(
                    DiagnosticCode::MissingRequiredProperty,
                    format!("Missing required property: {}", field_name),
                    format!("{}/{}", path, field_name),
                );
                return None;
            }
        };
        T::try_from_json(ctx, &format!("{}/{}", path, field_name), raw_value)
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
        let raw_value = match self.get(field_name) {
            Some(value) => value,
            None => return None, // Optional field is not present, so we just return None without pushing an error
        };
        T::try_from_json(ctx, &format!("{}/{}", path, field_name), raw_value)
    }

    /// Checks if the JSON object is a reference object, which is defined as an object that has a `$ref` property and no other properties.
    ///
    /// # Returns
    ///
    /// `true` if the JSON object is a reference object, or `false` if it is not.
    pub fn is_ref_object(&self) -> bool {
        self.contains_key("$ref") && self.0.len() == 1
    }

    /// If the JSON object is a reference object, tries to parse the `$ref` property as a `JsonRef`.
    /// If the `$ref` property is missing, `None` will be returned. If the `$ref` property is present but cannot be
    /// parsed into a valid `JsonRef`, an error will be pushed to the `ParserContext` and `None` will be returned.
    /// If the JSON object is not a reference object, `None` will be returned.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The parser context to which any diagnostic errors should be pushed, if found.
    /// * `path` - A JSON pointer string indicating the location of the value being parsed within the overall JSON structure.
    ///
    /// # Returns
    ///
    /// An `Option<JsonRef>` which is `Some(JsonRef)` if the JSON object is a reference object and the `$ref` property
    /// was successfully parsed into a `JsonRef`, or `None` otherwise.
    pub fn get_ref(&self, ctx: &mut ParserContext, path: &str) -> Option<JsonRef> {
        if self.is_ref_object() {
            // unwrap is safe here because we have already checked that the $ref field is present with is_ref_object
            let ref_str = self.required_field::<String>(ctx, path, "$ref").unwrap();

            if JsonPointer::is_valid_local_json_pointer(&ref_str) {
                Some(JsonRef::new_local_pointer(
                    ref_str.clone(),
                    JsonPointer::from(&ref_str),
                ))
            } else {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidReference,
                    format!("Invalid JSON pointer: {}", ref_str),
                    path.into(),
                );
                None
            }
        } else {
            None
        }
    }
}

impl<'a> TryFromJson<'a> for String {
    fn try_from_json(
        ctx: &mut ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        match value {
            serde_json::Value::String(s) => Some(s.to_owned()),
            _ => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidPropertyType,
                    format!("Expected a string, but found: {value}"),
                    path.into(),
                );
                None
            }
        }
    }
}

impl<'a> TryFromJson<'a> for bool {
    fn try_from_json(
        ctx: &mut ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        match value {
            serde_json::Value::Bool(b) => Some(*b),
            _ => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidPropertyType,
                    format!("Expected a boolean, but found: {value}"),
                    path.into(),
                );
                None
            }
        }
    }
}
