//! The `common` module contains common types and utilities that are used across different parts of the IR in the library.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use crate::ir::{TokenId, TokenPath};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TokenName(pub String);

impl Into<String> for TokenName {
    fn into(self) -> String {
        self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Deprecation {
    WithMessage(String),
    Boolean(bool),
}

/// Source metadata for a token after resolver merging.
///
/// `source` points to the originating resolver source reference (file path,
/// fragment, or inline source marker). `pointer` is the JSON pointer to the
/// token node within that source.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TokenProvenance {
    pub source: String,
    pub pointer: String,
}

/// Common metadata shared by token and group nodes in the parsed IR.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenCommon {
    pub id: TokenId,
    pub path: TokenPath,
    /// Optional source-level provenance emitted by the resolver and parsed from `$extensions`.
    pub provenance: Option<TokenProvenance>,
    pub name: TokenName,
    pub description: Option<String>,
    pub deprecation: Option<Deprecation>,
    pub extensions: Option<HashMap<String, serde_json::Value>>,
}

impl Hash for TokenCommon {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.id.hash(state);
        self.path.hash(state);
        self.provenance.hash(state);
        self.name.hash(state);
        self.description.hash(state);
        self.deprecation.hash(state);

        match &self.extensions {
            Some(extensions) => {
                true.hash(state);

                // Hash map entries in a stable order so hashing is deterministic.
                let mut entries: Vec<_> = extensions.iter().collect();
                entries.sort_unstable_by(|a, b| a.0.cmp(b.0));
                for (key, value) in entries {
                    key.hash(state);
                    value.hash(state);
                }
            }
            None => {
                false.hash(state);
            }
        }
    }
}

impl TokenCommon {
    /// Returns the canonical dot-path token name derived from the token path.
    pub fn canonical_name(&self) -> String {
        self.path.as_dot_path()
    }

    /// Returns a JSON pointer to the token node.
    pub fn json_pointer(&self) -> String {
        self.path.as_json_pointer()
    }

    /// Returns a JSON pointer to the token value (`$value`).
    pub fn value_json_pointer(&self) -> String {
        self.path.as_value_json_pointer()
    }
}

/// Parses a deprecation value from a JSON value.
///
/// Converts a `serde_json::Value` into a `Deprecation` enum variant.
/// Supports two formats:
/// - Boolean values are converted to `Deprecation::Boolean`
/// - String values are converted to `Deprecation::WithMessage`
///
/// # Arguments
///
/// * `value` - A reference to a `serde_json::Value` to parse
///
/// # Returns
///
/// Returns `Some(Deprecation)` if the value is a boolean or string,
/// otherwise returns `None` if the value is of an unsupported type.
pub fn parse_deprecation_value(value: &serde_json::Value) -> Option<Deprecation> {
    match value {
        serde_json::Value::Bool(b) => Some(Deprecation::Boolean(*b)),
        serde_json::Value::String(s) => Some(Deprecation::WithMessage(s.clone())),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_deprecation_value() {
        // Test parsing a boolean value
        let bool_value = serde_json::Value::Bool(true);
        let deprecation = parse_deprecation_value(&bool_value);
        assert_eq!(deprecation, Some(Deprecation::Boolean(true)));

        // Test parsing a string value
        let string_value = serde_json::Value::String("Deprecated".to_string());
        let deprecation = parse_deprecation_value(&string_value);
        assert_eq!(
            deprecation,
            Some(Deprecation::WithMessage("Deprecated".to_string()))
        );

        // Test parsing an unsupported value
        let null_value = serde_json::Value::Null;
        let deprecation = parse_deprecation_value(&null_value);
        assert_eq!(deprecation, None);
    }
}
