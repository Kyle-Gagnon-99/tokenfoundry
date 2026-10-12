//! The `lexer` module provides functionality for tokenizing JSON input while preserving span information.
//! It uses the `logos` crate for efficient lexing and defines a `Token` enum that represents the different types of tokens in JSON.

use chumsky::span::SimpleSpan;
use logos::Logos;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Logos)]
#[logos(skip r"[ \t\r\n\f]+")]
pub enum Token {
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

    #[regex(r#""([^"\\\x00-\x1F]|\\(["\\bnfrt/]|u[a-fA-F0-9]{4}))*""#, |lex| unescape(lex.slice()))]
    String(String),

    /// This is not converted to a number type yet, so that we can inspect the original string representation of the number if needed.
    #[regex(r"-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?", |lex| lex.slice().to_string())]
    Number(String),

    Error,
}

/// Lexes the input JSON string and returns an iterator over tokens along with their corresponding spans.
pub fn lex_json(input: &str) -> impl Iterator<Item = (Token, SimpleSpan)> {
    Token::lexer(input)
        .spanned()
        .map(|(token, span)| match token {
            // We are mapping Logos' span to Chumsky's SimpleSpan
            Ok(tok) => (tok, span.into()),
            Err(_) => (Token::Error, span.into()),
        })
}

/// Strips the surrounding quotes from a lexed JSON string literal and decodes its escape sequences.
/// The lexer regex already guarantees the escapes are well-formed. Lone surrogates become U+FFFD.
fn unescape(raw: &str) -> String {
    let inner = &raw[1..raw.len() - 1];
    let mut out = String::with_capacity(inner.len());
    let mut chars = inner.chars();

    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('b') => out.push('\u{8}'),
            Some('f') => out.push('\u{c}'),
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('u') => {
                let hi = read_hex4(&mut chars);
                if (0xD800..0xDC00).contains(&hi) {
                    let mut lookahead = chars.clone();
                    if lookahead.next() == Some('\\') && lookahead.next() == Some('u') {
                        let lo = read_hex4(&mut lookahead);
                        if (0xDC00..0xE000).contains(&lo) {
                            chars = lookahead;
                            let cp = 0x10000 + ((hi - 0xD800) << 10) + (lo - 0xDC00);
                            out.push(char::from_u32(cp).unwrap_or('\u{FFFD}'));
                            continue;
                        }
                    }
                }
                out.push(char::from_u32(hi).unwrap_or('\u{FFFD}'));
            }
            // `"`, `\\` and `/` map to themselves.
            Some(other) => out.push(other),
            None => {}
        }
    }
    out
}

fn read_hex4(chars: &mut std::str::Chars<'_>) -> u32 {
    (0..4).fold(0, |acc, _| {
        acc * 16 + chars.next().and_then(|c| c.to_digit(16)).unwrap_or(0)
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lex_string(input: &str) -> String {
        match lex_json(input).next() {
            Some((Token::String(s), _)) => s,
            other => panic!("expected string token, got {other:?}"),
        }
    }

    #[test]
    fn strips_quotes_and_decodes_escapes() {
        assert_eq!(lex_string(r#""$value""#), "$value");
        assert_eq!(lex_string(r#""a\nb\"c\\d\/e""#), "a\nb\"c\\d/e");
        assert_eq!(lex_string(r#""\u0041""#), "A");
        assert_eq!(lex_string(r#""\ud83d\ude00""#), "\u{1F600}");
        assert_eq!(lex_string(r#""\ud83d""#), "\u{FFFD}");
    }
}
