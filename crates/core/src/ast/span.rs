//! The `span` module in `ast` provides functionality for representing and working with spans within the abstract syntax tree (AST) specifically
//! related to DTCG AST. It uses spans to track the positions of various elements within the source code

use json_spanned::parser::Spanned;

use crate::source::{SourceId, SourceSpan};

/// A struct that associates a value of type `T` with a `SourceSpan` within the AST.
/// `SourceSpan` contains the associated ID of the source file and the range of characters within that source file that this AST element spans.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstSpanned<T> {
    pub value: T,
    pub span: SourceSpan,
}

impl<T> AstSpanned<T> {
    /// Creates a new `AstSpanned` instance with the given value and source span.
    pub fn new(value: T, span: SourceSpan) -> Self {
        Self { value, span }
    }

    /// Creates a new `AstSpanned` instance from a `Spanned` value and a `SourceId`.
    /// This method extracts the value and span from the `Spanned` instance and converts the span into a `SourceSpan` using the provided `SourceId`.
    pub fn from_spanned(spanned: Spanned<T>, source_id: SourceId) -> Self {
        Self {
            value: spanned.value,
            span: SourceSpan::from_span(source_id, spanned.span.into_range()),
        }
    }
}
