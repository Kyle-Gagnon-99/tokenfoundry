//! The `span` module provides functionality for handling source spans, which represent a range of characters within a source file.

use std::ops::Range;

use crate::source::SourceId;

/// A `Span` represents a range of characters within a source file, defined by a start and end position.
pub type Span = Range<usize>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSpan {
    pub source_id: SourceId,
    pub span: Span,
}

impl SourceSpan {
    /// Creates a new `SourceSpan` with the given `source_id`, `start`, and `end` positions.
    /// The `start` and `end` define the range of the span within the source file.
    pub fn new(source_id: SourceId, start: usize, end: usize) -> Self {
        Self {
            source_id,
            span: Span { start, end },
        }
    }

    /// Creates a new `SourceSpan` from an existing `Span` and a `source_id`.
    /// This is useful when you already have a `Span` and want to associate it with a specific source file.
    pub fn from_span(source_id: SourceId, span: Span) -> Self {
        Self { source_id, span }
    }

    pub fn from_json_span(source_id: SourceId, span: json_spanned::parser::Span) -> Self {
        Self {
            source_id,
            span: Span {
                start: span.start,
                end: span.end,
            },
        }
    }
}

/// A `Spanned<T>` struct that associates a value of type `T` with a `SourceSpan`.
#[derive(Debug, Clone)]
pub struct Spanned<T> {
    pub value: T,
    pub span: SourceSpan,
}

impl<T> Spanned<T> {
    pub fn new(value: T, span: SourceSpan) -> Self {
        Self { value, span }
    }
}

pub struct Property<T> {
    /// The key of the property, represented as a `Spanned<String>`.
    /// This allows us to track the source span of the key in the original source file.
    pub key: Spanned<String>,
    /// The value of the property, represented as a `Spanned<T>`.
    /// This allows us to track the source span of the value in the original source file.
    pub value: Spanned<T>,
    /// The source span of the entire property, which includes both the key and the value.
    pub span: SourceSpan,
}
