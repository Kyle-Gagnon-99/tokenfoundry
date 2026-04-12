//! The `utility` module contains utility functions for parsing and working with the IR

use crate::{
    ParserContext,
    ir::{JsonObject, RefOr, TryFromJson},
};

pub fn parse_ref_or_value<'a, T: TryFromJson<'a>>(
    ctx: &mut ParserContext,
    path: &str,
    value: &'a serde_json::Value,
) -> Option<RefOr<T>> {
    match JsonObject::from_value(value) {
        Some(obj) => {
            if obj.is_ref_object() {
                match obj.get_ref(ctx, path) {
                    Some(json_ref) => Some(RefOr::Ref(json_ref)),
                    None => None, // The error has already been pushed by get_ref, so we just return None here
                }
            } else {
                let parsed_value = T::try_from_json(ctx, path, value).map(RefOr::Literal);
                return parsed_value;
            }
        }
        None => {
            let parsed_value = T::try_from_json(ctx, path, value).map(RefOr::Literal);
            return parsed_value;
        }
    }
}

pub fn parse_value<'a, T: TryFromJson<'a>>(
    ctx: &mut ParserContext,
    path: &str,
    value: &'a serde_json::Value,
) -> Option<T> {
    T::try_from_json(ctx, path, value)
}
