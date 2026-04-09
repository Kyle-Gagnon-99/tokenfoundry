//! The `common` module contains common types and utilities that are used across different parts of the IR in the library.

use std::collections::HashMap;

use crate::ir::{DocumentId, TokenId, TokenPath};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Deprecation {
    WithMessage(String),
    Boolean(bool),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TokenCommon {
    pub id: TokenId,
    pub path: TokenPath,
    pub document_id: DocumentId,
    pub description: Option<String>,
    pub deprecation: Option<Deprecation>,
    pub extensions: HashMap<String, String>,
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
