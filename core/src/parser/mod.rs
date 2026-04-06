//! The `parser` module defines data structures and logic for parsing design tokens from various input formats, such as JSON

use std::collections::BTreeMap;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum RawNode {
    Token(RawToken),
    Group(RawGroup),
}

/// Represents the deprecation status of a token, which can be either a simple deprecation (indicated by a boolean `true`) or a deprecation with a message providing details about the deprecation (indicated by a string).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum JsonDeprecation {
    /// A simple deprecation without additional details, represented by a boolean `true` in the input
    Simple(bool),
    /// A deprecation with a message providing details about the deprecation, represented by a string in the input
    WithMessage(String),
}

/// This struct represents the common properties of all token nodes in the token tree,
/// such a the description, depcration, and other metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RawCommon {
    /// The description of the token, which can be used to provide additional context and information about the token's purpose and usage
    #[serde(rename = "$description")]
    pub description: Option<String>,

    /// The deprecation message for the token. The value may be a string providing details about the deprecation,
    /// or a boolean `true` indicating that the token is deprecated without providing additional details.
    #[serde(rename = "$deprecated")]
    pub deprecated: Option<JsonDeprecation>,

    /// A map of custom metadata properties for the token
    #[serde(rename = "$extensions")]
    pub extensions: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RawToken {
    #[serde(flatten)]
    pub common: RawCommon,

    #[serde(rename = "$value")]
    pub value: serde_json::Value,

    #[serde(rename = "$type")]
    pub token_type: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RawGroup {
    #[serde(flatten)]
    pub common: RawCommon,

    #[serde(flatten)]
    pub children: BTreeMap<String, RawNode>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raw_token_deserialization() {
        let json = r#"{
            "$description": "A sample token",
            "$deprecated": "This token is deprecated",
            "$extensions": {
                "customProperty": "customValue"
            },
            "$value": "red",
            "$type": "color"
        }"#;

        let token: RawToken = serde_json::from_str(json).unwrap();
        assert_eq!(token.common.description.unwrap().as_str(), "A sample token");
        assert_eq!(
            token.common.deprecated.unwrap(),
            JsonDeprecation::WithMessage(String::from("This token is deprecated"))
        );
        assert_eq!(
            token
                .common
                .extensions
                .unwrap()
                .get("customProperty")
                .unwrap()
                .as_str()
                .unwrap(),
            "customValue"
        );
        assert_eq!(token.value, serde_json::Value::String(String::from("red")));
        assert_eq!(token.token_type.unwrap(), String::from("color"));
    }

    #[test]
    fn test_raw_group_deserialization() {
        let json = r#"{
            "$description": "A sample group",
            "$deprecated": true,
            "$extensions": {
                "customProperty": "customValue"
            },
            "childToken": {
                "$value": "blue",
                "$type": "color"
            }        
        }"#;

        let group: RawGroup = serde_json::from_str(json).unwrap();
        assert_eq!(group.common.description.unwrap().as_str(), "A sample group");
        assert_eq!(
            group.common.deprecated.unwrap(),
            JsonDeprecation::Simple(true)
        );
        assert_eq!(
            group
                .common
                .extensions
                .unwrap()
                .get("customProperty")
                .unwrap()
                .as_str()
                .unwrap(),
            "customValue"
        );
        let child_token = match group.children.get("childToken").unwrap() {
            RawNode::Token(token) => token,
            _ => panic!("Expected a token node"),
        };
        assert_eq!(
            child_token.value,
            serde_json::Value::String(String::from("blue"))
        );
        assert_eq!(child_token.token_type.as_ref().unwrap(), "color");
    }
}
