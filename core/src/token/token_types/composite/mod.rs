//! The `composite` module defines the various composite design tokens per the DTCG specification, which are tokens that are composed of other tokens.

pub mod border;
pub mod gradient;
pub mod shadow;
pub mod stroke_style;
pub mod transition;
pub mod typography;

// Re-export all of the composite tokens
pub use border::*;
pub use gradient::*;
pub use shadow::*;
pub use stroke_style::*;
pub use transition::*;
pub use typography::*;

use crate::{
    ParserContext,
    ir::{JsonObject, JsonRef, TokenAlias, TryFromJson},
};

/// The `RefAliasOrLiteral` enum represents the ability for a property value in a composite
/// token, who can be a token, to be an alias to another token, a reference to another token,
/// or a literal value of type `T`.
///
/// This is different from `TokenValue<T>` in that Ref is an object ref rather than a
/// property string ref
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RefAliasOrLiteral<T> {
    Ref(JsonRef),
    Alias(TokenAlias),
    Literal(T),
}

impl<'a, T: TryFromJson<'a>> TryFromJson<'a> for RefAliasOrLiteral<T> {
    fn try_from_json(
        ctx: &mut ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        match value {
            serde_json::Value::String(s) => {
                // It is a string, so check if it is a DTCG reference
                if TokenAlias::is_valid_dtcg_alias(&s) {
                    return Some(RefAliasOrLiteral::Alias(
                        TokenAlias::from_dtcg_alias(s).unwrap(),
                    ));
                }

                // It is not valid, so try to parse it as a literal value
                T::try_from_json(ctx, path, value).map(RefAliasOrLiteral::Literal)
            }
            serde_json::Value::Object(_) => {
                let obj = JsonObject::from_value(value)?;
                if obj.is_ref_object() {
                    obj.get_ref(ctx, path).map(RefAliasOrLiteral::Ref)
                } else {
                    // It is an object, but not a reference object, so it must be a literal value
                    T::try_from_json(ctx, path, value).map(RefAliasOrLiteral::Literal)
                }
            }
            _ => {
                // It is neither a string nor an object, so it must be a literal value
                T::try_from_json(ctx, path, value).map(RefAliasOrLiteral::Literal)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AliasOrLiteral<T> {
    Alias(TokenAlias),
    Literal(T),
}

impl<'a, T: TryFromJson<'a>> TryFromJson<'a> for AliasOrLiteral<T> {
    fn try_from_json(
        ctx: &mut ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        match value {
            serde_json::Value::String(s) => {
                // It is a string, so check if it is a DTCG reference
                if TokenAlias::is_valid_dtcg_alias(&s) {
                    return Some(AliasOrLiteral::Alias(
                        TokenAlias::from_dtcg_alias(s).unwrap(),
                    ));
                }

                // It is not valid, so try to parse it as a literal value
                T::try_from_json(ctx, path, value).map(AliasOrLiteral::Literal)
            }
            _ => {
                // It is neither a string nor an object, so it must be a literal value
                T::try_from_json(ctx, path, value).map(AliasOrLiteral::Literal)
            }
        }
    }
}

pub fn parse_composite_token_field<'a, T: TryFromJson<'a>>(
    ctx: &mut ParserContext,
    path: &str,
    value: &'a serde_json::Value,
) -> Option<RefAliasOrLiteral<T>> {
    match value {
        serde_json::Value::String(s) => {
            // It is a string, so check if it is a DTCG reference
            if TokenAlias::is_valid_dtcg_alias(&s) {
                return Some(RefAliasOrLiteral::Alias(
                    TokenAlias::from_dtcg_alias(s).unwrap(),
                ));
            }

            // It is not valid, so try to parse it as a literal value
            T::try_from_json(ctx, path, value).map(RefAliasOrLiteral::Literal)
        }
        serde_json::Value::Object(_) => {
            let obj = JsonObject::from_value(value)?;
            if obj.is_ref_object() {
                obj.get_ref(ctx, path).map(RefAliasOrLiteral::Ref)
            } else {
                // It is an object, but not a reference object, so it must be a literal value
                T::try_from_json(ctx, path, value).map(RefAliasOrLiteral::Literal)
            }
        }
        _ => {
            // It is neither a string nor an object, so it must be a literal value
            T::try_from_json(ctx, path, value).map(RefAliasOrLiteral::Literal)
        }
    }
}

pub fn parse_alias_or_literal<'a, T: TryFromJson<'a>>(
    ctx: &mut ParserContext,
    path: &str,
    value: &'a serde_json::Value,
) -> Option<AliasOrLiteral<T>> {
    match value {
        serde_json::Value::String(s) => {
            // It is a string, so check if it is a DTCG reference
            if TokenAlias::is_valid_dtcg_alias(&s) {
                return Some(AliasOrLiteral::Alias(
                    TokenAlias::from_dtcg_alias(s).unwrap(),
                ));
            }

            // It is not valid, so try to parse it as a literal value
            T::try_from_json(ctx, path, value).map(AliasOrLiteral::Literal)
        }
        _ => {
            // It is neither a string nor an object, so it must be a literal value
            T::try_from_json(ctx, path, value).map(AliasOrLiteral::Literal)
        }
    }
}
