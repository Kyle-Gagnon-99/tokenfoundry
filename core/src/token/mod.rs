//! The `token` module provides functionality for parsing and handling design tokens.
pub mod ir;
pub mod token_types;
pub mod utils;

use std::collections::HashMap;

pub use token_types::*;

use crate::ParserContext;

pub enum ParseState<T> {
    Parsed(T),
    Skipped,
}

impl<T> Into<Option<T>> for ParseState<T> {
    fn into(self) -> Option<T> {
        match self {
            ParseState::Parsed(value) => Some(value),
            ParseState::Skipped => None,
        }
    }
}

/// Converts tokens from the raw JSON to the given struct
pub trait TryFromJson<'a>: Sized {
    fn try_from_json(
        ctx: &mut ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self>;
}

/// The value of deprecation can either be a boolean or a string message. If it's a boolean, it indicates whether the token is deprecated or not.
/// If it's a string message, it provides additional information about the deprecation, such as the reason for deprecation or the recommended alternative.
pub enum DeprecationValue {
    /// Indicates that the token is deprecated with an additional message providing more information about the deprecation.
    WithMessage(String),
    /// Indicates that the token is deprecated without providing additional information.
    Boolean(bool),
}

/// Represents a node in the token hierarchy, which can be either a token or a group of tokens.
pub enum Node {
    Token(Token),
    Group(Group),
}

/// Represents the common properties of both tokens and groups, such as name, description, extensions, and deprecation status.
pub struct NodeCommon {
    pub name: String,
    pub description: Option<String>,
    pub extensions: HashMap<String, serde_json::Value>,
    pub deprecated: Option<DeprecationValue>,
}

/// Represents a design token, which has a path, type, and value. The path is a unique identifier for the token,
/// which can be used to reference it from other tokens or groups.
pub struct Token {
    pub common: NodeCommon,
    pub path: String,
    pub token_type: TokenType,
    pub value: TokenValue,
}

/// Represents a group of tokens, which can contain both tokens and other groups. The group has a common property that includes
/// the name, description, extensions, and deprecation status of the group.
pub struct Group {
    pub common: NodeCommon,
    pub children: HashMap<String, Node>,
}
