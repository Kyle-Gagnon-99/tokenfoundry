//! The `token` module provides the basic abstract syntax tree (AST) representation of individual tokens in the authored DTCG.
//!
//! This is the final shape: it holds copied values and spans, and does not need the arena or the lowered tree to be
//! read. Every token and group has a [`TokenId`], and references between tokens use it.

use indexmap::IndexMap;

use crate::ast::{
    node::{Node, PropId},
    types::{TokenType, TokenValue},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct TokenId(pub usize);

/// How a token's value is given.
#[derive(Debug, Clone, PartialEq)]
pub enum RefAliasOrValue<T> {
    /// `"$value": "{other.token}"`, which is always a whole token.
    Alias(TokenId),
    /// A literal.
    Value(T),
    /// `{"$ref": ...}` or a token-level `$ref`. It holds the final literal the reference leads to.
    Reference(PropId),
}

/// Every token and group of a compilation. A [`TokenId`] is an index into `tokens`.
#[derive(Debug, Default)]
pub struct TokenArena {
    pub tokens: Vec<AnyToken>,
    pub root: TokenId,
    /// The token or group each lowered prop became, for mapping diagnostics and references back to tokens.
    pub by_prop: IndexMap<PropId, TokenId>,
}

impl TokenArena {
    pub fn get(&self, id: TokenId) -> &AnyToken {
        &self.tokens[id.0]
    }

    /// Finds a token or group by walking member names from the root, like `["color", "brand", "$root"]`.
    pub fn find(&self, path: &[&str]) -> Option<TokenId> {
        path.iter()
            .try_fold(self.root, |id, name| match self.get(id) {
                AnyToken::Group(group) => group.value.tokens.get(*name).copied(),
                _ => None,
            })
    }

    /// Everything that is a usable token, skipping groups and tokens that failed validation.
    pub fn tokens(&self) -> impl Iterator<Item = (TokenId, &Node<Token>)> {
        self.tokens.iter().enumerate().filter_map(|(i, t)| match t {
            AnyToken::Token(token) => Some((TokenId(i), token)),
            _ => None,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Deprecation {
    WithMessage(String),
    Boolean(bool),
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct TokenCommon {
    pub description: Option<Node<String>>,
    pub deprecation: Option<Node<Deprecation>>,
    /// Vendor data is opaque JSON, so only the handle is kept.
    pub extensions: Option<PropId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub common: TokenCommon,
    pub token_type: TokenType,
    pub value: Node<RefAliasOrValue<TokenValue>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GroupToken {
    pub common: TokenCommon,
    /// Child groups and tokens by name, in order. A `$root` token is under the key `$root`.
    pub tokens: IndexMap<String, TokenId>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AnyToken {
    Token(Node<Token>),
    Group(Node<GroupToken>),
    /// A token that had a problem. The diagnostics say what, and this keeps its id and span.
    Invalid(Node<()>),
}
