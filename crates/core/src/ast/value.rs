//! The `value` module in `ast` provides functionality and structs for representing and working with values within the abstract syntax tree (AST) specifically related to DTCG AST.

use json_spanned::parser::JsonNumber;

use crate::{ast::span::AstSpanned, source::SourceSpan};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DtcgValueSyntax {
    /// Represents a `null` value within the AST. This is not significant or valid in the context of DTCG, but used to
    /// maintain consistency with the JSON structure and for potential error handling during parsing.
    Null,
    /// Represents a boolean value within the AST.
    Bool(bool),
    /// Represents a numeric value within the AST.
    Number(JsonNumber),
    /// Represents a string value within the AST.
    String(String),
    /// Represents an array value within the AST, containing a list of other `DtcgValueSyntax` elements.
    Array(Vec<DtcgValueSyntax>),
    /// Represents an object value within the AST, containing a list of `DtcgObjectSyntax` elements.
    Object(Vec<DtcgObjectSyntax>),
    /// Represents an invalid value within the AST. This can be used to mark values that failed validation or parsing.
    Invalid,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DtcgObjectSyntax {
    /// The members of the object, each represented as a `DtcgMemberSyntax` within the AST.
    /// An array is used instead of a map because the order of members may be significant and duplicate keys are "allowed" in the AST
    /// (but they will be validated later in the process and skipped)
    pub members: Vec<DtcgMemberSyntax>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DtcgMemberSyntax {
    /// The key of the member, represented as a spanned string within the AST.
    pub key: AstSpanned<String>,
    /// The value of the member, represented as a spanned value within the AST.
    pub value: AstSpanned<DtcgValueSyntax>,
    /// The span of the entire member within the source code, represented as a `SourceSpan`.
    pub span: SourceSpan,
}
