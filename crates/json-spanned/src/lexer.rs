//! The `lexer` module provides functionality for tokenizing JSON input while preserving span information.
//! It uses the `logos` crate for efficient lexing and defines a `Token` enum that represents the different types of tokens in JSON.

use chumsky::span::SimpleSpan;
use logos::Logos;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Logos)]
#[logos(skip r"[ \t\r\n\f]+")]
pub enum Token<'source> {
    #[token("false", |_| false)]
    #[token("true", |_| true)]
    Boolean(bool),

    #[token("{")]
    BraceOpen,

    #[token("}")]
    BraceClose,

    #[token("[")]
    BracketOpen,

    #[token("]")]
    BracketClose,

    #[token(":")]
    Colon,

    #[token(",")]
    Comma,

    #[token("null")]
    Null,

    #[regex(r#""([^"\\\x00-\x1F]|\\(["\\bnfrt/]|u[a-fA-F0-9]{4}))*""#, |lex| lex.slice())]
    String(&'source str),

    /// This is not converted to a number type yet, so that we can inspect the original string representation of the number if needed.
    #[regex(r"-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?", |lex| lex.slice())]
    Number(&'source str),

    Error,
}

/// Lexes the input JSON string and returns an iterator over tokens along with their corresponding spans.
pub fn lex_json<'source>(
    input: &'source str,
) -> impl Iterator<Item = (Token<'source>, SimpleSpan)> {
    Token::lexer(input)
        .spanned()
        .map(|(token, span)| match token {
            // We are mapping Logos' span to Chumsky's SimpleSpan
            Ok(tok) => (tok, span.into()),
            Err(_) => (Token::Error, span.into()),
        })
}
