//! The `node` module contains the definitions for what a node is in the IR and the parsed DTCG format.

use crate::ir::{DocumentId, JsonRef, TokenAlias, TokenCommon};

/// The `TokenValue` enum represents the source of a token's value in the IR, which can either be a literal value of type `T`,
/// an alias to another token or a reference to another token.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenValue {
    Value(IrTokenValue),
    Alias(TokenAlias),
    Ref(JsonRef),
}

/// The `IrTokenType` enum represents the different tokens.
#[derive(Debug, Clone, PartialEq)]
pub enum IrTokenValue {}

#[derive(Debug, Clone, PartialEq)]
pub enum IrTokenType {}

#[derive(Debug, Clone, PartialEq)]
pub struct IrToken {
    pub common: TokenCommon,
    pub token_type: IrTokenType,
    pub value: TokenValue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IrGroupToken<'a> {
    pub common: TokenCommon,
    pub children: Vec<&'a IrNode<'a>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IrDocument<'a> {
    pub id: DocumentId,
    pub tokens: Vec<IrNode<'a>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IrNode<'a> {
    Token(IrToken),
    Group(IrGroupToken<'a>),
}
