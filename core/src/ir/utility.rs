//! The `utility` module contains utility functions for parsing and working with the IR

use crate::{
    ParserContext,
    errors::DiagnosticCode,
    ir::{JsonPointer, JsonRef, RefOr},
    token::{
        TryFromJsonField,
        utils::{require_object_field, require_string},
    },
};

pub fn parse_ref_or_value<'a, T: TryFromJsonField<'a>>(
    ctx: &mut ParserContext,
    path: &str,
    value: &'a serde_json::Value,
) -> Option<RefOr<T>> {
    match value {
        serde_json::Value::Object(map) => {
            // Check to see if the object has a $ref property and no other properties
            // We may have a $ref, but if there are other properties, this is not a valid reference
            // The specification does not explicitly state that a reference object cannot have other properties, but we
            // will consider it invalid to avoid ambiguity and potential errors in parsing.
            let has_ref = map.contains_key("$ref");

            if has_ref && map.len() == 1 {
                // This is a reference object
                let raw_ref = match require_object_field(ctx, path, map, "$ref") {
                    Some(value) => value,
                    None => {
                        // The error has already been pushed by require_object_field, so we just return None here
                        return None;
                    }
                };
                let ref_str = match require_string(ctx, path, raw_ref) {
                    Some(value) => value,
                    None => {
                        // The error has already been pushed by require_string, so we just return None here
                        return None;
                    }
                };

                // Check if the reference is a valid JSON pointer
                if JsonPointer::is_valid_local_json_pointer(ref_str) {
                    return Some(RefOr::Ref(JsonRef::new_local_pointer(
                        ref_str.to_owned(),
                        JsonPointer::from(ref_str),
                    )));
                } else {
                    ctx.push_to_errors(
                        DiagnosticCode::InvalidReference,
                        format!("Invalid JSON pointer: {}", ref_str),
                        path.into(),
                    );
                    return None;
                }
            } else {
                // This is a regular object that should be parsed as the type T
                let parsed_value = T::try_from_json_field(ctx, path, value).map(RefOr::Literal);
                return parsed_value;
            }
        }
        _ => {
            let parsed_value = T::try_from_json_field(ctx, path, value).map(RefOr::Literal);
            return parsed_value;
        }
    }
}

pub fn parse_ref_or_value_with_parser<'a, T>(
    ctx: &mut ParserContext,
    path: &str,
    value: &'a serde_json::Value,
    parser: impl Fn(&mut ParserContext, &str, &'a serde_json::Value) -> Option<T>,
) -> Option<RefOr<T>> {
    match value {
        serde_json::Value::Object(map) => {
            // Check to see if the object has a $ref property and no other properties
            // We may have a $ref, but if there are other properties, this is not a valid reference
            // The specification does not explicitly state that a reference object cannot have other properties, but we
            // will consider it invalid to avoid ambiguity and potential errors in parsing.
            let has_ref = map.contains_key("$ref");

            if has_ref && map.len() == 1 {
                // This is a reference object
                let raw_ref = match require_object_field(ctx, path, map, "$ref") {
                    Some(value) => value,
                    None => {
                        // The error has already been pushed by require_object_field, so we just return None here
                        return None;
                    }
                };
                let ref_str = match require_string(ctx, path, raw_ref) {
                    Some(value) => value,
                    None => {
                        // The error has already been pushed by require_string, so we just return None here
                        return None;
                    }
                };

                // Check if the reference is a valid JSON pointer
                if JsonPointer::is_valid_local_json_pointer(ref_str) {
                    return Some(RefOr::Ref(JsonRef::new_local_pointer(
                        ref_str.to_owned(),
                        JsonPointer::from(ref_str),
                    )));
                } else {
                    ctx.push_to_errors(
                        DiagnosticCode::InvalidReference,
                        format!("Invalid JSON pointer: {}", ref_str),
                        path.into(),
                    );
                    return None;
                }
            } else {
                // This is a regular object that should be parsed using the provided parser function
                return parser(ctx, path, value).map(RefOr::Literal);
            }
        }
        _ => {
            // This is a primitive value that should be parsed using the provided parser function
            return parser(ctx, path, value).map(RefOr::Literal);
        }
    }
}

pub fn parse_value<'a, T: TryFromJsonField<'a>>(
    ctx: &mut ParserContext,
    path: &str,
    value: &'a serde_json::Value,
) -> Option<T> {
    T::try_from_json_field(ctx, path, value)
}
