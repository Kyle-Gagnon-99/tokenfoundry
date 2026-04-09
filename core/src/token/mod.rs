//! The `token` module provides functionality for parsing and handling design tokens.
pub mod token_types;
pub mod utils;

use crate::{
    ParserContext,
    errors::DiagnosticCode,
    ir::{JsonPointer, JsonRef, TokenAlias, TokenValue},
};

pub enum ParseState<T> {
    Parsed(T),
    Skipped,
}

impl<T> Into<Option<T>> for ParseState<T> {
    fn into(self) -> Option<T> {
        match self {
            ParseState::Parsed(value) => Some(value),
            ParseState::Skipped => None,
        }
    }
}

impl<T> ParseState<T> {
    pub fn map<U>(self, f: impl Fn(T) -> U) -> ParseState<U> {
        match self {
            ParseState::Parsed(value) => ParseState::Parsed(f(value)),
            ParseState::Skipped => ParseState::Skipped,
        }
    }
}

/// Converts tokens from the raw JSON to the given struct
pub trait TryFromJson<'a>: Sized {
    fn try_from_json(
        ctx: &mut ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self>;
}

pub trait TryFromJsonField<'a>: Sized {
    fn try_from_json_field(
        ctx: &mut ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self>;
}

pub fn parse_plain_token<'a, T: TryFromJson<'a>>(
    ctx: &mut ParserContext,
    path: &str,
    value: &'a serde_json::Value,
) -> ParseState<TokenValue<T>> {
    T::try_from_json(ctx, path, value).map(TokenValue::Value)
}

pub fn parse_token<'a, T: TryFromJson<'a>>(
    ctx: &mut ParserContext,
    path: &str,
    value: &'a serde_json::Value,
) -> ParseState<TokenValue<T>> {
    match value {
        serde_json::Value::String(str_val) => {
            if let Some(alias) = TokenAlias::from_dtcg_alias(str_val) {
                ParseState::Parsed(TokenValue::Alias(alias))
            } else {
                parse_plain_token(ctx, path, value)
            }
        }
        serde_json::Value::Object(map) => {
            let raw_ref = match utils::require_object_field(ctx, path, map, "$ref") {
                Some(value) => value,
                None => return ParseState::Skipped,
            };

            let ref_str = match utils::require_string(ctx, path, raw_ref) {
                Some(value) => value,
                None => return ParseState::Skipped,
            };

            if !JsonPointer::is_valid_local_json_pointer(ref_str) {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidReference,
                    format!("Invalid JSON pointer: {}", ref_str),
                    path.into(),
                );
                return ParseState::Skipped;
            }

            ParseState::Parsed(TokenValue::Ref(JsonRef::new_local_pointer(
                ref_str.to_owned(),
                JsonPointer::from(ref_str),
            )))
        }
        _ => parse_plain_token(ctx, path, value),
    }
}
