//! The `json-spanned` crate provides an AST for JSON that preserves the original source code's span information.
//! This is useful for applications that need to provide detailed error messages or perform transformations on JSON data while
//! maintaining a connection to the original source code.

use chumsky::{Parser, error::Rich, input::IterInput, span::SimpleSpan, span::Span};

use crate::{
    lexer::{Token, lex_json},
    parser::{JsonValue, Spanned, parser},
};

pub mod lexer;
pub mod parser;

/// Parses a JSON string and returns a `Spanned<JsonValue>` if successful, or a vector of `Rich` errors if parsing fails.
/// `parse_json` is a zero-copy parser, so the `'source` lifetime is tied to the input string.
///
/// # Arguments
///
/// * `input`: A string slice containing the JSON data to be parsed.
///
/// # Returns
///
/// The function returns a `Result` which is:
/// - `Ok(Spanned<JsonValue<'source>>)` if parsing is successful, where `Spanned<JsonValue<'source>>` contains the parsed JSON value along with its span information.
/// - `Err(Vec<Rich<'source, Token<'source>>>)` if parsing fails, where `Vec<Rich<'source, Token<'source>>>` contains detailed error information about the parsing failure.
pub fn parse_json<'source>(
    input: &'source str,
) -> Result<Spanned<JsonValue<'source>>, Vec<Rich<'source, Token<'source>>>> {
    let tokens = lex_json(input).collect::<Vec<_>>();
    parser()
        .parse(IterInput::new(
            tokens.into_iter(),
            SimpleSpan::new((), input.len()..input.len()),
        ))
        .into_result()
}
