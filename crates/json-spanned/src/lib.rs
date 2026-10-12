//! The `json-spanned` crate provides an AST for JSON that preserves the original source code's span information.
//! This is useful for applications that need to provide detailed error messages or perform transformations on JSON data while
//! maintaining a connection to the original source code.

use chumsky::{Parser, error::Rich, input::IterInput, prelude::end, span::SimpleSpan, span::Span};

use crate::{
    lexer::{Token, lex_json},
    parser::{JsonValue, Spanned, parser},
};

pub mod lexer;
pub mod parser;

/// Parses a JSON string, recovering from syntax errors where possible.
///
/// Returns the (possibly partial) tree together with every error found. The tree is `Some` whenever
/// recovery produced one, so callers should check the error list rather than the `Option` to
/// decide whether the input was valid. Content after the root value is reported as an error.
///
/// # Arguments
///
/// * `input`: A string slice containing the JSON data to be parsed.
pub fn parse_json(input: &str) -> (Option<Spanned<JsonValue>>, Vec<Rich<'_, Token>>) {
    let tokens = lex_json(input).collect::<Vec<_>>();
    parser()
        .then_ignore(end())
        .parse(IterInput::new(
            tokens.into_iter(),
            SimpleSpan::new((), input.len()..input.len()),
        ))
        .into_output_errors()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_input_has_no_errors() {
        let (value, errors) = parse_json(r#"{"$value": 1}"#);
        assert!(errors.is_empty());
        let JsonValue::Object(members) = value.unwrap().value else {
            panic!("expected object");
        };
        assert_eq!(members[0].key.value, "$value");
    }

    #[test]
    fn trailing_content_is_an_error() {
        let (_, errors) = parse_json("{} garbage");
        assert!(!errors.is_empty());
    }

    #[test]
    fn recovery_returns_partial_tree_and_errors() {
        let (value, errors) = parse_json(r#"{"a": 1, "b": [1,, 2]}"#);
        assert!(value.is_some());
        assert!(!errors.is_empty());
    }

    #[test]
    fn unterminated_string_is_an_error() {
        let (_, errors) = parse_json(r#"{"a": "oops}"#);
        assert!(!errors.is_empty());
    }
}
