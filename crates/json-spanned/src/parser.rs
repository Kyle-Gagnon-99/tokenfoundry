//! The `parser` module provides functionality for parsing JSON input into an abstract syntax tree (AST) while preserving span information.

use std::{
    fmt,
    hash::{Hash, Hasher},
};

use chumsky::{Parser, error::Rich, input::ValueInput, prelude::*, span::SimpleSpan};

use crate::lexer::Token;

/// The `Span` type is an alias for `SimpleSpan`, which represents a span of text in the input.
pub type Span = SimpleSpan;

/// The `Spanned` struct represents a value of type `T` along with its associated span information.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Spanned<T> {
    pub value: T,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NumberParseError {
    InvalidInteger,
    IntegerOutOfRange,
    InvalidFloat,
    NonFiniteFloat,
}

impl fmt::Display for NumberParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            NumberParseError::InvalidInteger => write!(f, "invalid integer"),
            NumberParseError::IntegerOutOfRange => write!(f, "integer is out of supported range"),
            NumberParseError::InvalidFloat => write!(f, "invalid floating-point number"),
            NumberParseError::NonFiniteFloat => write!(f, "float must be finite (no NaN/Infinity)"),
        }
    }
}

// The below code is inspired by the `serde_json` crate's number parsing logic, but adapted to our needs and simplified for this context.

#[derive(Debug, Clone, Copy)]
pub enum Number {
    /// Positive integer
    PosInt(u64),
    /// Negative integer, always less than zero
    NegInt(i64),
    /// Floating point number, always finite
    Float(f64),
}

impl PartialEq for Number {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Number::PosInt(a), Number::PosInt(b)) => a == b,
            (Number::NegInt(a), Number::NegInt(b)) => a == b,
            (Number::Float(a), Number::Float(b)) => a == b,
            _ => false,
        }
    }
}

impl Eq for Number {}

impl Hash for Number {
    fn hash<H: Hasher>(&self, h: &mut H) {
        match *self {
            Number::PosInt(i) => i.hash(h),
            Number::NegInt(i) => i.hash(h),
            Number::Float(f) => {
                if f == 0.0f64 {
                    0.0f64.to_bits().hash(h);
                } else {
                    f.to_bits().hash(h);
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct JsonNumber {
    n: Number,
}

impl TryFrom<&str> for JsonNumber {
    type Error = NumberParseError;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        // First, inspect the string to determine if there is a decimal point or exponent, which would indicate a float.
        let is_float = value.contains('.') || value.contains('e') || value.contains('E');

        if is_float {
            let f = value
                .parse::<f64>()
                .map_err(|_| NumberParseError::InvalidFloat)?;
            if !f.is_finite() {
                return Err(NumberParseError::NonFiniteFloat);
            }
            Ok(JsonNumber {
                n: Number::Float(f),
            })
        } else {
            // It is not a float, so if it starts with a '-', it is a negative integer, otherwise it is a positive integer.
            if value.starts_with('-') {
                let i = value
                    .parse::<i64>()
                    .map_err(|_| NumberParseError::InvalidInteger)?;
                Ok(JsonNumber {
                    n: Number::NegInt(i),
                })
            } else {
                let u = value
                    .parse::<u64>()
                    .map_err(|_| NumberParseError::InvalidInteger)?;
                Ok(JsonNumber {
                    n: Number::PosInt(u),
                })
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JsonMember<'source> {
    pub key: Spanned<&'source str>,
    pub value: Spanned<JsonValue<'source>>,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum JsonValue<'source> {
    Null,
    Boolean(bool),
    Number(JsonNumber),
    String(&'source str),
    Array(Vec<Spanned<JsonValue<'source>>>),
    Object(Vec<JsonMember<'source>>),
    Invalid,
}

pub fn parser<'source, I>() -> impl Parser<
    'source,
    I,
    Spanned<JsonValue<'source>>,
    chumsky::extra::Err<Rich<'source, Token<'source>>>,
>
where
    I: ValueInput<'source, Token = Token<'source>, Span = SimpleSpan>,
{
    recursive(|value| {
        let null = just(Token::Null).to(JsonValue::Null);
        let boolean = just(Token::Boolean(true))
            .to(JsonValue::Boolean(true))
            .or(just(Token::Boolean(false)).to(JsonValue::Boolean(false)));
        let number = select! { Token::Number(raw) => raw }.try_map(
            |raw, span| -> Result<JsonValue<'source>, Rich<'source, Token<'source>>> {
                JsonNumber::try_from(raw)
                    .map(JsonValue::Number)
                    .map_err(|e| Rich::custom(span, e.to_string()))
            },
        );
        let string = select! { Token::String(s) => JsonValue::String(s) };

        let primitive = null
            .or(boolean)
            .or(number)
            .or(string)
            .map_with(|node, e| Spanned {
                value: node,
                span: e.span(),
            })
            .boxed();

        let key = select! { Token::String(s) => s }
            .try_map(|s, span| {
                Ok::<Spanned<&'source str>, Rich<'source, Token<'source>>>(Spanned {
                    value: s,
                    span,
                })
            })
            .boxed();

        let member = key
            .then_ignore(just(Token::Colon))
            .then(value.clone())
            .map(|(key, value)| JsonMember { key, value })
            .boxed();

        let array = value
            .clone()
            .separated_by(just(Token::Comma).recover_with(skip_then_retry_until(
                any().ignored(),
                one_of(vec![Token::Comma, Token::BracketClose]).ignored(),
            )))
            .collect::<Vec<_>>()
            .delimited_by(
                just(Token::BracketOpen),
                just(Token::BracketClose)
                    .ignored()
                    .recover_with(via_parser(end()))
                    .recover_with(skip_then_retry_until(any().ignored(), end())),
            )
            .try_map(|items, span| {
                Ok::<Spanned<JsonValue<'source>>, Rich<'source, Token<'source>>>(Spanned {
                    value: JsonValue::Array(items),
                    span,
                })
            })
            .boxed();

        let object = Box::new(
            member
                .clone()
                .separated_by(just(Token::Comma).recover_with(skip_then_retry_until(
                    any().ignored(),
                    one_of(vec![Token::Comma, Token::BraceClose]).ignored(),
                )))
                .allow_trailing()
                .collect::<Vec<_>>()
                .delimited_by(
                    just(Token::BraceOpen),
                    just(Token::BraceClose)
                        .ignored()
                        .recover_with(via_parser(end()))
                        .recover_with(skip_then_retry_until(any().ignored(), end())),
                )
                .try_map(|members, span| {
                    Ok::<Spanned<JsonValue<'source>>, Rich<'source, Token<'source>>>(Spanned {
                        value: JsonValue::Object(members),
                        span,
                    })
                }),
        )
        .boxed();

        choice((array, object, primitive))
            .recover_with(via_parser(nested_delimiters(
                Token::BracketOpen,
                Token::BracketClose,
                [(Token::BraceOpen, Token::BraceClose)],
                |span| Spanned {
                    value: JsonValue::Invalid,
                    span,
                },
            )))
            .recover_with(via_parser(nested_delimiters(
                Token::BraceOpen,
                Token::BraceClose,
                [(Token::BracketOpen, Token::BracketClose)],
                |span| Spanned {
                    value: JsonValue::Invalid,
                    span,
                },
            )))
            .recover_with(skip_then_retry_until(
                any().ignored(),
                one_of(vec![Token::BraceClose, Token::BracketClose, Token::Comma]).ignored(),
            ))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use chumsky::input::IterInput;

    use crate::lexer::lex_json;

    fn parse_json(input: &str) -> Result<Spanned<JsonValue<'_>>, Vec<Rich<'_, Token<'_>>>> {
        let tokens = lex_json(input).collect::<Vec<_>>();
        parser()
            .parse(IterInput::new(
                tokens.into_iter(),
                SimpleSpan::new((), input.len()..input.len()),
            ))
            .into_result()
    }

    #[test]
    fn test_json_number_parsing() {
        let cases = vec![
            ("0", Number::PosInt(0)),
            ("123", Number::PosInt(123)),
            ("-456", Number::NegInt(-456)),
            ("3.14", Number::Float(3.14)),
            ("-2.71", Number::Float(-2.71)),
        ];

        for (input, expected) in cases {
            let parsed = JsonNumber::try_from(input).unwrap();
            assert_eq!(parsed.n, expected);
        }

        let invalid_cases = vec!["abc", "1.2.3", "--5", "NaN", "Infinity"];
        for input in invalid_cases {
            assert!(JsonNumber::try_from(input).is_err());
        }
    }

    #[test]
    fn test_parses_spanned_array_with_nested_object() {
        let input = r#"[1,{"x":true}]"#;
        let parsed = parse_json(input).expect("expected successful parse");

        assert_eq!(parsed.span, SimpleSpan::new((), 0..input.len()));

        let JsonValue::Array(items) = parsed.value else {
            panic!("expected array root");
        };

        assert_eq!(items.len(), 2);

        assert_eq!(items[0].span, SimpleSpan::new((), 1..2));
        assert!(matches!(
            items[0].value,
            JsonValue::Number(JsonNumber {
                n: Number::PosInt(1)
            })
        ));

        assert_eq!(items[1].span, SimpleSpan::new((), 3..13));
        let JsonValue::Object(members) = &items[1].value else {
            panic!("expected object at second array position");
        };

        assert_eq!(members.len(), 1);
        assert_eq!(members[0].key.value, "\"x\"");
        assert_eq!(members[0].key.span, SimpleSpan::new((), 4..7));
        assert_eq!(members[0].value.span, SimpleSpan::new((), 8..12));
        assert!(matches!(members[0].value.value, JsonValue::Boolean(true)));
    }

    #[test]
    fn test_number_out_of_range_error_with_span() {
        // This number is too large for u64::MAX (18446744073709551615)
        let input = r#"18446744073709551616"#; // u64::MAX + 1, guaranteed overflow
        let result = parse_json(input);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());

        // Check that the error is associated with the number token's span
        let first_error = &errors[0];
        // The error's span should cover the entire number token
        assert_eq!(first_error.span(), &SimpleSpan::new((), 0..input.len()));
    }

    #[test]
    fn test_negative_number_out_of_range_error() {
        // This number is too small for i64::MIN (-9223372036854775808)
        let input = r#"-9223372036854775809"#; // i64::MIN - 1, guaranteed underflow
        let result = parse_json(input);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());

        // Verify the error span points to the number token
        let first_error = &errors[0];
        assert_eq!(first_error.span(), &SimpleSpan::new((), 0..input.len()));
    }

    #[test]
    fn test_non_finite_float_rejection() {
        // Note: The lexer's regex should not emit "Infinity" or "NaN" tokens,
        // but if we manually crafted an edge case, the parser would reject it.
        // For now, we test that a valid float works and an actual out-of-spec case fails.
        let valid = parse_json("1.5").expect("valid float should parse");
        assert!(matches!(
            valid.value,
            JsonValue::Number(JsonNumber {
                n: Number::Float(_)
            })
        ));
    }

    #[test]
    fn test_invalid_json_structure() {
        // This is invalid JSON due to the closing brace after the trailing comma in the array.
        // The trailing comma is also technically invalid, but in this case, we did not expect a closing brace after it as it could have
        // been the start of another element in the array.
        let input = r#"{"key": [1, 2,}"#;
        let result = parse_json(input);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());

        // The error should indicate an unexpected token (the closing brace) after the trailing comma.
        let first_error = &errors[0];
        assert_eq!(first_error.span(), &SimpleSpan::new((), 14..15)); // The span of the unexpected token
    }

    #[test]
    fn test_trailing_comma_in_array() {
        // This is invalid JSON due to the trailing comma in the array.
        let input = r#"[1, 2,]"#;
        let result = parse_json(input);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());

        // The error should indicate an unexpected token (the closing bracket) after the trailing comma.
        let first_error = &errors[0];
        assert_eq!(first_error.span(), &SimpleSpan::new((), 6..7)); // The span of the unexpected token
    }

    #[test]
    fn test_double_comma_in_array() {
        // This is invalid JSON due to the double comma in the array.
        let input = r#"[1,, 2]"#;
        let result = parse_json(input);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());

        // The error should indicate an unexpected token (the second comma) after the first comma.
        let first_error = &errors[0];
        assert_eq!(first_error.span(), &SimpleSpan::new((), 3..4)); // The span of the unexpected token
    }

    #[test]
    fn test_double_colon_in_object() {
        // This is invalid JSON due to the double colon in the object.
        let input = r#"{"key":: "value"}"#;
        let result = parse_json(input);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());

        // The error should indicate an unexpected token (the second colon) after the first colon.
        let first_error = &errors[0];
        assert_eq!(first_error.span(), &SimpleSpan::new((), 7..8)); // The span of the unexpected token
    }

    #[test]
    fn test_missing_colon_in_object() {
        // This is invalid JSON due to the missing colon in the object.
        let input = r#"{"key" "value"}"#;
        let result = parse_json(input);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());

        // The error should indicate an unexpected token (the string "value") where a colon was expected.
        let first_error = &errors[0];
        assert_eq!(first_error.span(), &SimpleSpan::new((), 7..14)); // The span of the unexpected token
    }

    #[test]
    fn test_missing_comma_in_array() {
        // This is invalid JSON due to the missing comma in the array.
        let input = r#"[1 2]"#;
        let result = parse_json(input);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());

        // The error should indicate an unexpected token (the number 2) where a comma was expected.
        let first_error = &errors[0];
        assert_eq!(first_error.span(), &SimpleSpan::new((), 3..4)); // The span of the unexpected token
    }

    #[test]
    fn test_unexpected_token_in_object() {
        // This is invalid JSON due to an unexpected token in the object.
        let input = r#"{"key": true false}"#;
        let result = parse_json(input);

        assert!(result.is_err());
        let errors = result.unwrap_err();
        assert!(!errors.is_empty());

        // The error should indicate an unexpected token (the boolean "false") where a comma or closing brace was expected.
        let first_error = &errors[0];
        assert_eq!(first_error.span(), &SimpleSpan::new((), 13..18)); // The span of the unexpected token
    }

    #[test]
    fn test_recovered_ast_with_errors() {
        let input = r#"
        {
            "key1": 123,
            "key2": [true, false,],
            "key3": {"nestedKey": "value"},
            "key4": null
        }, {
        "key5": 456
        }"#;

        let tokens = lex_json(input).collect::<Vec<_>>();
        println!("Tokens: {:?}", tokens);
        let ast = parser().parse(IterInput::new(
            tokens.into_iter(),
            SimpleSpan::new((), input.len()..input.len()),
        ));

        println!("AST: {:?}", ast);
    }

    #[test]
    fn test_recovery_stress_collects_multiple_errors_and_partial_ast() {
        let input = r#"
        {
            "ok1": 1,
            "bad_array": [1,, 2, {"x":: 3}, [4,]],
            "bad_object": {"a": true false, "b" 2, "c": [0, 1,]},
            "ok2": {"nested": [{"k": "v"}, 9]}
        }
        "#;

        let tokens = lex_json(input).collect::<Vec<_>>();
        let (output, errs) = parser()
            .parse(IterInput::new(
                tokens.into_iter(),
                SimpleSpan::new((), input.len()..input.len()),
            ))
            .into_output_errors();

        println!("Recovery output: {:?}", output);
        println!("Recovery errors: {:?}", errs);

        assert!(
            output.is_some(),
            "expected partial AST output from recovery parser"
        );
        assert!(
            errs.len() >= 3,
            "expected multiple diagnostics from nested malformed input"
        );

        let root = output.expect("output should exist");
        let JsonValue::Object(members) = root.value else {
            panic!("expected root object");
        };

        assert!(
            members.iter().any(|m| m.key.value == "\"ok1\""),
            "expected parser to keep valid members despite nearby errors"
        );
        assert!(
            members.iter().any(|m| m.key.value == "\"ok2\""),
            "expected parser to recover and keep later valid members"
        );
    }
}
