//! The `node` module contains the definitions for what a node is in the IR and the parsed DTCG format.

use crate::ir::{DocumentId, JsonRef, TokenAlias, TokenCommon, TokenId};

/// The `TokenValue` enum represents the source of a token's value in the IR, which can either be a literal value of type `T`,
/// an alias to another token or a reference to another token.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum TokenValue<T> {
    Value(T),
    Alias(TokenAlias),
    Ref(JsonRef),
}

/// The `IrTokenType` enum represents the different tokens.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IrTokenType {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrToken {
    pub common: TokenCommon,
    pub value: TokenValue<IrTokenType>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrGroupToken {
    pub common: TokenCommon,
    pub children: Vec<TokenId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IrDocument {
    pub id: DocumentId,
    pub tokens: Vec<TokenId>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IrNode {
    Token(IrToken),
    Group(IrGroupToken),
}
