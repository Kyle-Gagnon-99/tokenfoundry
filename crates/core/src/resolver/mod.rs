//! The `resolver` module is responsible for using DTCG's resolver. Documentation on how to use the resolver can be found
//! [here](https://www.designtokens.org/tr/2025.10/resolver/)
//! The resolver is responsible for resolving design tokens according to the rules defined in the DTCG specification. Resolvers can be used to compute
//! the final values of design tokens based on their definitions and relationships with other tokens.

use std::collections::HashMap;

use json_spanned::parser::JsonValue;

pub static DEFAULT_RESOLVER_VERSION: &str = "2025.10";

pub struct Set {
    /// The description of the set.
    pub description: Option<String>,
    /// The tokens contained in the set.
    pub tokens: Vec<JsonValue>,
}

/// The resolver configuration contains the configurations used to generate the final resolved design tokens.
pub struct ResolverConfig {
    /// A short, human-readable name for the document. Not required.
    pub name: Option<String>,
    /// The version of the resolver to use. Required.
    pub version: String,
    /// A human-readable description for this document. Not required.
    pub description: Option<String>,
    /// Definition of sets
    pub sets: Option<HashMap<String, JsonValue>>,
    /// Definition of modifiers
    pub modifiers: Option<HashMap<String, JsonValue>>,
}
