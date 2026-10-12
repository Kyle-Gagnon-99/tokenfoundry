//! The `node` module in `ast` provides the definitions and types for nodes used in the abstract syntax tree (AST) representation of DTCG.
//! Nodes can be either a token or a group. A token represents a single value, while a group represents a collection of tokens or other groups.
//! Typically tokens and groups share the same metadata types (such as description, type, and so on), so this module consolidates the definitions of these
//! shared types.

use std::collections::HashMap;

use json_spanned::parser::JsonMember;

//use crate::ast::token::DtcgTypeSyntax;

/// CommonNodeProperties contains the common properties shared by both tokens and groups in DTCG.
/// These properties include description, deprecation status, token type, and any extensions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommonNodeProperties {
    pub description: Option<String>,
    pub deprecated: Option<bool>,
    //pub token_type: Option<DtcgTypeSyntax>,
    pub extensions: Option<HashMap<String, JsonMember>>,
}
