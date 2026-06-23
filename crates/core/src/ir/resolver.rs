//! The `resolver` module contains the intermediate representation for the Design Tokens Resolver specification (2025.10)
//!
//! This module provides data structures to represent:
//! - Resolver documents with sets, modifiers, and resolution order
//! - Sets as named collections of token sources
//! - Modifiers for conditional token values with contexts
//! - Reference objects for JSON pointers and file references
//! - Resolution inputs and validation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a complete resolver document per DTCG 2025.10 specification
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolverDocument {
    /// Optional human-readable name for the resolver document
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,

    /// Required version string. Must be "2025.10"
    pub version: String,

    /// Optional description providing additional context
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Optional JSON Schema URL
    #[serde(rename = "$schema", skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,

    /// Defined sets that can be referenced in resolution order
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sets: Option<HashMap<String, Set>>,

    /// Defined modifiers for conditional token inclusion
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub modifiers: Option<HashMap<String, Modifier>>,

    /// Resolution order: array of sets and modifiers that determine final token composition
    #[serde(rename = "resolutionOrder", skip_serializing_if = "Option::is_none")]
    pub resolution_order: Option<Vec<ResolutionItem>>,

    /// Vendor-specific extensions (e.g., "figma.com", "adobe.com")
    #[serde(rename = "$extensions", skip_serializing_if = "Option::is_none")]
    pub extensions: Option<HashMap<String, serde_json::Value>>,

    /// JSON Schema $defs for bundled files (optional, tool-specific)
    #[serde(rename = "$defs", skip_serializing_if = "Option::is_none")]
    pub defs: Option<HashMap<String, serde_json::Value>>,
}

/// Represents a set: a named collection of design token sources
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Set {
    /// Optional human-readable description of the set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Array of token sources to merge in order (later overrides earlier)
    pub sources: Vec<TokenSource>,

    /// Vendor-specific extensions
    #[serde(rename = "$extensions", skip_serializing_if = "Option::is_none")]
    pub extensions: Option<HashMap<String, serde_json::Value>>,
}

/// Represents a modifier: conditional token sets with multiple contexts
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Modifier {
    /// Optional human-readable description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,

    /// Map of context names to their corresponding token sources
    /// Must have 2+ entries (tools should warn if only 1)
    pub contexts: HashMap<String, Vec<TokenSource>>,

    /// Optional default context name (must match a key in contexts)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,

    /// Vendor-specific extensions
    #[serde(rename = "$extensions", skip_serializing_if = "Option::is_none")]
    pub extensions: Option<HashMap<String, serde_json::Value>>,
}

/// Represents a token source: either a JSON reference or inline tokens
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TokenSource {
    /// Reference to tokens via $ref (JSON Pointer or file path)
    Reference {
        #[serde(rename = "$ref")]
        ref_path: String,
        #[serde(flatten)]
        overrides: Option<serde_json::Value>,
    },
    /// Inline token definitions
    Inline(serde_json::Value),
}

/// Represents an item in the resolution order
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ResolutionItem {
    /// Reference to a set or modifier
    Reference {
        #[serde(rename = "$ref")]
        ref_path: String,
    },
    /// Inline set definition
    InlineSet {
        #[serde(rename = "type")]
        item_type: String, // "set"
        name: String,
        description: Option<String>,
        sources: Vec<TokenSource>,
        #[serde(rename = "$extensions", skip_serializing_if = "Option::is_none")]
        extensions: Option<HashMap<String, serde_json::Value>>,
    },
    /// Inline modifier definition
    InlineModifier {
        #[serde(rename = "type")]
        item_type: String, // "modifier"
        name: String,
        description: Option<String>,
        contexts: HashMap<String, Vec<TokenSource>>,
        #[serde(skip_serializing_if = "Option::is_none")]
        default: Option<String>,
        #[serde(rename = "$extensions", skip_serializing_if = "Option::is_none")]
        extensions: Option<HashMap<String, serde_json::Value>>,
    },
}

impl ResolverDocument {
    /// Get the list of modifier names
    ///
    /// # Returns
    ///
    /// A vector of modifier names defined in the resolver document, or an empty vector if no modifiers are defined.
    pub fn get_modifier_names(&self) -> Vec<String> {
        self.modifiers
            .as_ref()
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    }

    /// Get a map of modifier names to their available context values
    ///
    /// # Returns
    ///
    /// A HashMap where the keys are modifier names and the values are vectors of context names available for each modifier.
    /// If no modifiers are defined, returns an empty HashMap.
    pub fn get_modifier_map(&self) -> HashMap<String, Vec<String>> {
        self.modifiers
            .as_ref()
            .map(|m| {
                m.iter()
                    .map(|(name, modifier)| {
                        (name.clone(), modifier.contexts.keys().cloned().collect())
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

/// Represents a JSON reference/pointer
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonPointerRef {
    /// The reference string (e.g., "#/sets/base" or "path/to/file.json" or "path/file.json#/property")
    pub pointer: String,
    /// Extracted fragment identifier after # (if present)
    pub fragment: Option<String>,
    /// File path portion (if referencing external file)
    pub file_path: Option<String>,
}

impl JsonPointerRef {
    /// Parse a reference string into its components
    pub fn parse(pointer: &str) -> Self {
        if let Some(hash_idx) = pointer.find('#') {
            let (file_part, fragment_part) = pointer.split_at(hash_idx);
            let file_path = if file_part.is_empty() {
                None
            } else {
                Some(file_part.to_string())
            };
            Self {
                pointer: pointer.to_string(),
                fragment: Some(fragment_part[1..].to_string()), // Skip the '#'
                file_path,
            }
        } else if pointer.starts_with('#') {
            // Same-document reference
            Self {
                pointer: pointer.to_string(),
                fragment: Some(pointer[1..].to_string()), // Skip the '#'
                file_path: None,
            }
        } else {
            // External file reference without fragment
            Self {
                pointer: pointer.to_string(),
                fragment: None,
                file_path: Some(pointer.to_string()),
            }
        }
    }

    /// Check if this is a same-document reference (starts with #)
    pub fn is_same_document(&self) -> bool {
        self.pointer.starts_with('#')
    }

    /// Check if this is a reference to a set (#/sets/...)
    pub fn is_set_reference(&self) -> bool {
        self.fragment
            .as_ref()
            .map(|f| f.starts_with("/sets/"))
            .unwrap_or(false)
    }

    /// Check if this is a reference to a modifier (#/modifiers/...)
    pub fn is_modifier_reference(&self) -> bool {
        self.fragment
            .as_ref()
            .map(|f| f.starts_with("/modifiers/"))
            .unwrap_or(false)
    }

    /// Check if this is a reference to resolution order (#/resolutionOrder/...)
    pub fn is_resolution_order_reference(&self) -> bool {
        self.fragment
            .as_ref()
            .map(|f| f.starts_with("/resolutionOrder/"))
            .unwrap_or(false)
    }

    /// Extract the name from a reference (e.g., "base" from "#/sets/base")
    pub fn extract_name(&self) -> Option<String> {
        self.fragment.as_ref().and_then(|f| {
            let parts: Vec<&str> = f.split('/').collect();
            if parts.len() >= 3 {
                Some(parts[2].to_string())
            } else {
                None
            }
        })
    }
}

/// Represents resolver inputs: modifier context selections
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ResolutionInput {
    /// Map of modifier names to their selected context values
    pub selections: HashMap<String, String>,
}

impl ResolutionInput {
    /// Create a new empty resolution input
    pub fn new() -> Self {
        Self {
            selections: HashMap::new(),
        }
    }

    /// Add a modifier context selection
    pub fn add_selection(&mut self, modifier_name: String, context_value: String) {
        self.selections.insert(modifier_name, context_value);
    }

    /// Validate input against a resolver document
    /// Returns errors if invalid
    pub fn validate(&self, resolver: &ResolverDocument) -> Vec<String> {
        let mut errors = Vec::new();

        let modifiers = match resolver.modifiers.as_ref() {
            Some(m) => m,
            None => {
                if !self.selections.is_empty() {
                    errors.push("Input provided but resolver has no modifiers".to_string());
                }
                return errors;
            }
        };

        // Check each input selection
        for (modifier_name, context_value) in &self.selections {
            if let Some(modifier) = modifiers.get(modifier_name) {
                if !modifier.contexts.contains_key(context_value) {
                    errors.push(format!(
                        "Invalid context '{}' for modifier '{}'",
                        context_value, modifier_name
                    ));
                }
            } else {
                errors.push(format!("Unknown modifier '{}'", modifier_name));
            }
        }

        // Check for missing required modifiers (those without defaults)
        for (modifier_name, modifier) in modifiers {
            if modifier.default.is_none() && !self.selections.contains_key(modifier_name) {
                errors.push(format!("Missing required modifier '{}'", modifier_name));
            }
        }

        errors
    }
}

impl Default for ResolutionInput {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_pointer_ref_same_document() {
        let ref_obj = JsonPointerRef::parse("#/sets/base");
        assert!(ref_obj.is_same_document());
        assert!(ref_obj.is_set_reference());
        assert_eq!(ref_obj.extract_name(), Some("base".to_string()));
    }

    #[test]
    fn test_json_pointer_ref_modifier() {
        let ref_obj = JsonPointerRef::parse("#/modifiers/theme");
        assert!(ref_obj.is_modifier_reference());
        assert_eq!(ref_obj.extract_name(), Some("theme".to_string()));
    }

    #[test]
    fn test_json_pointer_ref_external_file() {
        let ref_obj = JsonPointerRef::parse("path/to/tokens.json");
        assert!(!ref_obj.is_same_document());
        assert_eq!(ref_obj.file_path, Some("path/to/tokens.json".to_string()));
    }

    #[test]
    fn test_json_pointer_ref_external_with_fragment() {
        let ref_obj = JsonPointerRef::parse("path/to/tokens.json#/color");
        assert_eq!(ref_obj.file_path, Some("path/to/tokens.json".to_string()));
        assert_eq!(ref_obj.fragment, Some("/color".to_string()));
    }
}
