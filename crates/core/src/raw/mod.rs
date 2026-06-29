//! The `raw` module provides serde deserialization of the DTCG specification. It does not validate or transform the data,
//! but rather provides a type-safe representation of the raw data.

pub mod tokens;

// This is organized by "bottom-up" where the lowest-level types are defined first, and then higher-level types are built on top of them
// Rust handles forward references by default, but this organization makes it easier to read

use serde::{Deserialize, Deserializer};

// Helper types
/// Represents a DTCG alias, which is a string that can be used to reference another token in the DTCG specification.
/// By specification, it must be in the form `{token_name}` where `token_name` is the name of the token being referenced (using dot notation)
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DtcgAlias {
    pub raw: String,
}

impl DtcgAlias {
    /// Returns true if the given string is DTCG alias-like, meaning it starts with `{` and ends with `}` and has at least one character in between.
    /// This is a check to see if a string is likely to be a DTCG alias, but it does not guarantee that the string is a valid alias according to the DTCG specification.
    ///
    /// # Arguments
    ///
    /// * `value` - The string to check.
    ///
    /// # Returns
    ///
    /// * `true` if the string is DTCG alias-like, `false` otherwise.
    pub fn is_dtcg_alias_like(value: &str) -> bool {
        value.starts_with('{') && value.ends_with('}') && value.len() > 2
    }
}

/// Represents a JSON reference, which is a string that can be used to reference another token in the DTCG specification.
/// By specification, it must be in the form `#/path/to/token` where `path/to/token` is the path to the token being referenced
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JsonReference {
    pub raw: String,
}

impl JsonReference {
    /// Returns true if the given string is JSON reference-like, meaning it starts with `#/` or `/`.
    /// This is a check to see if a string is likely to be a JSON reference, but it does not guarantee that the string is a valid reference according to the DTCG specification.
    ///
    /// # Arguments
    ///
    /// * `value` - The string to check.
    ///
    /// # Returns
    ///
    /// * `true` if the string is JSON reference-like, `false` otherwise.
    pub fn is_json_reference_like(value: &str) -> bool {
        value.starts_with("#/") || value.starts_with('/')
    }
}

/// Represents either a DTCG alias or a literal value of type `T`. This is used to represent values that can be either
/// a reference to another token (via an alias) or a literal value (e.g., a color value, a number, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AliasOrLiteral<T> {
    Alias(DtcgAlias),
    Literal(T),
}

/// Represents either a JSON reference or a literal value of type `T`. This is used to represent values that can be either
/// a reference to another token (via a JSON reference) or a literal value (e.g., a color value, a number, etc.).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RefOrLiteral<T> {
    Ref(JsonReference),
    Literal(T),
}

impl<'de, T> Deserialize<'de> for AliasOrLiteral<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Deserialize the value as is
        let value = serde_json::Value::deserialize(deserializer)?;

        // Next, if the value is a string, check if it is alias-like
        // If it is, return an Alias variant, otherwise, attempt to deserialize it as a literal value of type T as normal
        if let serde_json::Value::String(s) = &value {
            if DtcgAlias::is_dtcg_alias_like(s) {
                return Ok(Self::Alias(DtcgAlias { raw: s.clone() }));
            }
        }

        // Otherwise, if the value is not alias-like, attempt to deserialize it as a literal value of type T
        T::deserialize(value)
            .map(Self::Literal)
            .map_err(serde::de::Error::custom)
    }
}

impl<'de, T> Deserialize<'de> for RefOrLiteral<T>
where
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Deserialize the value as is
        let value = serde_json::Value::deserialize(deserializer)?;

        // Next, if the value is a string, check if it is reference-like
        // If it is, return a Ref variant, otherwise, attempt to deserialize it as a literal value of type T as normal
        if let serde_json::Value::String(s) = &value {
            if JsonReference::is_json_reference_like(s) {
                return Ok(Self::Ref(JsonReference { raw: s.clone() }));
            }
        }

        // Otherwise, if the value is not reference-like, attempt to deserialize it as a literal value of type T
        T::deserialize(value)
            .map(Self::Literal)
            .map_err(serde::de::Error::custom)
    }
}
