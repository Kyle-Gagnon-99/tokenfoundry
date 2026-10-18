//! The `node` module in `ast` provides the definitions and types for nodes used in the abstract syntax tree (AST) representation of DTCG.
//! Nodes can be either a token or a group. A token represents a single value, while a group represents a collection of tokens or other groups.
//! Typically tokens and groups share the same metadata types (such as description, type, and so on), so this module consolidates the definitions of these
//! shared types.

use crate::{ir::lower::PropId, source::SourceSpan};

pub struct Node<T> {
    pub value: T,
    pub id: PropId,
    pub span: SourceSpan,
}
