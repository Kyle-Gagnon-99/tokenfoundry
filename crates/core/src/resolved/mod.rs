//! The `resolved` module contains the structures for representing resolved tokens that come from the IR and are ready to be transformed into various output formats.

use std::collections::BTreeMap;

use crate::ir::TokenPath;

pub mod token;
pub mod utils;

#[derive(Debug, Clone, thiserror::Error)]
pub enum ResolutionError {
    #[error("Unresolved alias: {0}")]
    UnresolvedAlias(String),
}

pub trait ToResolvedTokenValue<T> {
    fn to_resolved_token(&self) -> Result<T, ResolutionError>;
}

pub enum ResolvedTokenValue {
    Color,
}

/// Represents the value of a resolved token, which can either be a literal value or an alias to another token
///
/// JSON references should be resolved, as the resolved layer deals with tokens that are ready to be transformed into output formats,
/// and in those formats, we want to have the actual value.
pub enum ResolvedValue {
    Literal(ResolvedTokenValue),
    Alias(TokenPath),
}

/// Represents either an alias or a literal value. Used for composite token properties, in which the value can be a literal or an alias to another token.
/// Uses `T` to constrain the literal value to a specific type, such as `Color`, `Dimension`, etc rather than using `ResolvedTokenValue`, which is an enum of all possible token types.
pub enum AliasOrLiteral<T> {
    Alias(TokenPath),
    Literal(T),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResolvedTokenType {
    Color,
    Dimension,
    FontFamily,
    FontWeight,
    Duration,
    CubicBezier,
    Number,
    StrokeStyle,
    Border,
    Transition,
    Shadow,
    Gradient,
    Typography,
}

/// Represents a resolved token, which has a name, a value (which can be a literal or an alias), and metadata
pub struct ResolvedToken {
    pub path: TokenPath,
    pub token_type: ResolvedTokenType,
    pub value: ResolvedValue,
    pub description: Option<String>,
    pub extensions: BTreeMap<String, serde_json::Value>,
}

/// Represents a resolved group, which has a name, and metadata. Groups don't have values, as they are just a way to group tokens together.
pub struct ResolvedGroup {
    pub path: TokenPath,
    pub description: Option<String>,
    pub extensions: BTreeMap<String, serde_json::Value>,
}

pub struct ResolvedDocument {
    pub tokens: Vec<ResolvedToken>,
    pub groups: Vec<ResolvedGroup>,
}
