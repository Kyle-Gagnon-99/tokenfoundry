//! The `reference` module contains the defintions and logic for referencing and aliasing

use crate::{
    errors::DiagnosticCode,
    ir::{ParseState, TokenPath, TryFromJson},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TokenAlias {
    pub raw_value: String,
    pub target_path: TokenPath,
}

impl TokenAlias {
    /// Creates a new `TokenAlias` with the provided raw value and target path
    ///
    /// # Arguments
    ///
    /// - `raw_value` - A string representing the raw value of the alias, which may include references to other tokens
    /// - `target_path` - A `TokenPath` struct representing the path to the target token that this alias references
    ///
    /// # Returns
    ///
    /// A `TokenAlias` struct containing the provided raw value and target path, which can be used to represent an alias for a design token in the IR
    pub fn new(raw_value: String, target_path: TokenPath) -> Self {
        Self {
            raw_value,
            target_path,
        }
    }

    /// Checks if the provided raw value is a valid DTCG alias by verifying that it starts with '{' and ends with '}'
    ///
    /// # Arguments
    ///
    /// - `raw_value` - A string representing the raw value to be checked for validity as a DTCG alias
    ///
    /// # Returns
    ///
    /// A boolean value indicating whether the provided raw value is a valid DTCG alias (true) or not (false)
    pub fn is_valid_dtcg_alias(raw_value: &str) -> bool {
        // Check if the value starts with '{' and ends with '}', which is the expected format for a DTCG alias
        raw_value.starts_with('{') && raw_value.ends_with('}')
    }

    /// Creates a new `TokenAlias` from a raw value string that is expected to be in the DTCG alias format (e.g., "{group1.subgroupA.tokenX}")
    ///
    /// # Arguments
    ///
    /// - `raw_value` - A string representing the raw value of the alias, which should be in the format of a DTCG alias (starting with '{' and ending with '}')
    ///
    /// # Returns
    ///
    /// An `Option<TokenAlias>` which will contain a `TokenAlias` struct if the provided raw value is a valid DTCG alias, or `None` if the raw value is not valid
    ///
    /// # Examples
    ///
    /// ```
    /// use tokenfoundry_core::ir::TokenAlias;
    ///
    /// let raw_value = "{group1.subgroupA.tokenX}".to_string();
    /// if let Some(alias) = TokenAlias::from_dtcg_alias(&raw_value) {
    ///     println!("Created TokenAlias: {:?}", alias);
    /// } else {
    ///     println!("Invalid DTCG alias format");
    /// }
    /// ```
    pub fn from_dtcg_alias(raw_value: &str) -> Option<Self> {
        if Self::is_valid_dtcg_alias(raw_value) {
            // Extract the content inside the curly braces
            let inner_content = raw_value.trim_matches(|c| c == '{' || c == '}');
            // Split the inner content by '.' to get the segments of the path
            let segments: Vec<String> = inner_content.split('.').map(|s| s.to_string()).collect();
            // Create a TokenPath from the segments
            let target_path = TokenPath::from_segment_vec(segments);
            // Create and return a TokenAlias with the original raw value and the extracted target path
            Some(Self::new(raw_value.to_string(), target_path))
        } else {
            // If the raw value is not a valid DTCG alias, return None
            None
        }
    }
}

impl<'a> TryFromJson<'a> for TokenAlias {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        // Check if it is a string, if so, continue, if not, probably not meant for us, so return NoMatch and let other parsers try to parse it
        let s = match value {
            serde_json::Value::String(s) => s,
            _ => return ParseState::NoMatch,
        };

        // It is a string, so check if it starts with '{', if so continue, if not, probably not meant for us, so return NoMatch and let other parsers try to parse it
        if !s.starts_with('{') {
            return ParseState::NoMatch;
        }

        // If it does not end with '}', then it is not a valid alias, so push an error to the context and return Invalid
        // If it is a string that starts with '{' it is likely meant to be a DTCG alias, so we should push an error if it does not end with '}'
        if !s.ends_with('}') {
            ctx.push_to_errors(
                DiagnosticCode::InvalidReference,
                format!("Invalid DTCG alias format: {}", s),
                path.into(),
            );
            return ParseState::Invalid;
        }

        // It is a string that starts with '{' and ends with '}', so we will try to parse it as a DTCG alias
        match Self::from_dtcg_alias(s) {
            Some(alias) => ParseState::Parsed(alias),
            None => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidReference,
                    format!("Invalid DTCG alias format: {}", s),
                    path.into(),
                );
                return ParseState::Invalid;
            }
        }
    }
}

/// The `JsonPointer` struct represents a JSON Pointer, which is a string syntax for identifying a specific value within a JSON document.
/// It consists of a sequence of segments, where each segment represents a key in an object or an index in an array.
/// The segments are ordered from the root of the JSON document to the specific value, allowing for easy traversal
/// and referencing of values within the JSON document.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JsonPointer {
    pub segments: Vec<String>,
}

impl JsonPointer {
    /// Creates a new `JsonPointer` from a vector of strings representing the segments of the pointer
    ///
    /// # Returns
    ///
    /// A `JsonPointer` struct containing the provided segments, which can be used to identify a specific value within a JSON document.
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    /// Creates a new `JsonPointer` from an iterator of items that can be converted into strings, representing the segments of the pointer
    ///
    /// # Arguments
    ///
    /// * `segments` - An iterator of items that can be converted into strings, representing the segments of the pointer, ordered from the root of the JSON document to the specific value
    ///
    /// # Returns
    ///
    /// A `JsonPointer` struct containing the provided segments, which can be used to identify a specific value within a JSON document.
    ///
    /// # Examples
    ///
    /// ```
    /// use tokenfoundry_core::ir::JsonPointer;
    /// let pointer = JsonPointer::from_segments(vec!["group1", "subgroupA", "tokenX"]);
    /// ```
    pub fn from_segments<I, S>(segments: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self {
            segments: segments.into_iter().map(Into::into).collect(),
        }
    }

    /// Checks if the `JsonPointer` is a root pointer, which means it has no segments and points to the root of the JSON document
    ///
    /// # Returns
    ///
    /// `true` if the `JsonPointer` has no segments and points to the root of the JSON document, `false` otherwise.
    pub fn is_root(&self) -> bool {
        self.segments.is_empty()
    }

    /// Converts the `JsonPointer` into its string representation, which is a JSON Pointer string that can be used to reference a specific value within a JSON document.
    /// The string representation of a JSON Pointer consists of segments separated by '/', with a leading '/' to indicate the
    /// root of the JSON document. For example, a `JsonPointer` with segments ["group1", "subgroupA", "tokenX"] would be represented as "/group1/subgroupA/tokenX".
    /// If the `JsonPointer` has no segments (i.e., it is a root pointer), it is represented as "/".
    ///
    /// # Returns
    ///
    /// A string representation of the `JsonPointer` that can be used to reference a specific value within a JSON document.
    /// For example, a `JsonPointer` with segments ["group1", "subgroupA", "tokenX"] would be represented as "/group1/subgroupA/tokenX", and a root pointer
    /// with no segments would be represented as "/".
    pub fn to_string(&self) -> String {
        if self.segments.is_empty() {
            "/".to_string() // The root pointer is represented as "/"
        } else {
            format!("/{}", self.segments.join("/"))
        }
    }
}

/// The `JsonRefKind` enum represents the different kinds of JSON references that can be used in the IR to reference other tokens or values
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JsonRef {
    pub document: Option<String>,
    pub pointer: JsonPointer,
}

impl JsonRef {
    fn parse_local_pointer(pointer_str: &str) -> Option<JsonPointer> {
        // Check if the pointer string starts with a '/'. If not, we will add it to ensure it is a valid JSON pointer format
        let pointer_str = if pointer_str.starts_with('/') {
            pointer_str.to_string()
        } else {
            format!("/{}", pointer_str)
        };

        // Split the pointer string by '/' to get the individual segments of the JSON pointer
        let segments: Vec<String> = pointer_str
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| s.to_string())
            .collect();

        // Create a JsonPointer from the segments
        Some(JsonPointer::from_segments(segments))
    }

    /// Parses a JSON reference string into a `JsonRef` struct, if the string is in a valid format.
    /// The expected format for the reference string is either a JSON pointer with URI fragment support:
    ///     - `#/path/to/value` (for references within the same document)
    ///     - `/path/to/value` (for references within the same document, without the leading '#')
    ///     - `document.json#/path/to/value` (for references to another document, with the document name followed by a JSON pointer)
    ///     - `document.json` (for references to another document, without a JSON pointer, which would point to the root of that document)
    ///
    /// # Arguments
    ///
    /// - `ref_str` - A string representing the JSON reference to be parsed into a `JsonRef` struct
    ///
    /// # Returns
    ///
    /// An `Option<JsonRef>` which will contain a `JsonRef` struct if the provided reference string is in a valid format and can be successfully parsed, or `None` if the
    /// reference string is not in a valid format and cannot be parsed into a `JsonRef` struct.
    pub fn parse(ref_str: &str) -> Option<Self> {
        // First, check if the reference string starts with a '#' character, which indicates that it is a JSON pointer reference within the same document
        if ref_str.starts_with('#') {
            // Remove the leading '#' character to get the JSON pointer string
            let pointer_str = &ref_str[1..];

            // Create a JsonPointer from the segments
            let pointer = Self::parse_local_pointer(pointer_str)?;

            // Return a JsonRef with no document (since it's a reference within the same document) and the parsed pointer
            Some(JsonRef {
                document: None,
                pointer,
            })
        } else if ref_str.contains('#') {
            // It contains a '#' character, which indicates that it is a reference
            // Split on '#'. If there are more than 2 parts, it's an invalid format
            let parts: Vec<&str> = ref_str.split('#').collect();
            if parts.len() != 2 {
                return None;
            }

            let document = parts[0].to_string();
            let pointer_str = parts[1];
            let pointer = Self::parse_local_pointer(pointer_str)?;
            Some(JsonRef {
                document: Some(document),
                pointer,
            })
        } else if ref_str.starts_with('/') {
            // It starts with a '/', which indicates that it is a JSON pointer reference within the same document (without the leading '#')
            let pointer_str = ref_str;
            let pointer = Self::parse_local_pointer(pointer_str)?;
            Some(JsonRef {
                document: None,
                pointer,
            })
        } else {
            // It does not contain a '#' character and does not start with a '/', which indicates that it is a reference to another document without a JSON pointer (pointing to the root of that document)
            Some(JsonRef {
                document: Some(ref_str.to_string()),
                pointer: JsonPointer::new(), // An empty JsonPointer represents the root of the document
            })
        }
    }
}

impl<'a> TryFromJson<'a> for JsonRef {
    fn try_from_json(
        _ctx: &mut crate::ParserContext,
        _path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        let ref_val = match value {
            serde_json::Value::String(s) => s,
            _ => return ParseState::NoMatch,
        };

        let json_ref = match Self::parse(ref_val) {
            Some(json_ref) => json_ref,
            None => return ParseState::NoMatch,
        };

        ParseState::Parsed(json_ref)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JsonRefObject {
    pub reference: JsonRef,
}

impl<'a> TryFromJson<'a> for JsonRefObject {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        let json_ref_obj = match value {
            serde_json::Value::Object(obj) => obj,
            _ => return ParseState::NoMatch,
        };

        // If the object does not contain a "$ref" property, then it is not a JsonRefObject
        if !json_ref_obj.contains_key("$ref") {
            return ParseState::NoMatch;
        }

        // It does contain a "$ref" property, but if there are other properties in the object, then it is an invalid format for a JsonRefObject
        // so we will push an error to the context and return Invalid
        if json_ref_obj.len() != 1 {
            ctx.push_to_errors(
                DiagnosticCode::InvalidReference,
                format!("Invalid JsonRefObject format: expected only a '$ref' property, but found additional properties: {:?}", json_ref_obj),
                path.into(),
            );
            return ParseState::Invalid;
        }

        // It is an object that contains only a "$ref" property, so we will try to parse the value of the "$ref" property as a JsonRef
        let ref_value = &json_ref_obj["$ref"];
        match JsonRef::try_from_json(ctx, &format!("{}/{}", path, "$ref"), ref_value) {
            ParseState::Parsed(json_ref) => ParseState::Parsed(Self {
                reference: json_ref,
            }),
            ParseState::Invalid => ParseState::Invalid,
            ParseState::NoMatch => ParseState::Invalid, // If the value of the "$ref" property cannot be parsed as a JsonRef, then it is an invalid format for a JsonRefObject, so we will return Invalid
        }
    }
}

/// The `RefOr` enum represents a property value that can either be a literal value of type `T` or a reference
/// to another token or value in the IR using a `JsonRef`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RefOrLiteral<T> {
    Literal(T),
    Ref(JsonRefObject),
}

impl<T> RefOrLiteral<T> {
    /// Creates a new `RefOrLiteral` instance representing a literal value of type `T`
    ///
    /// # Arguments
    ///
    /// - `value` - The literal value of type `T` to be wrapped in the `RefOrLiteral` enum
    ///
    /// # Returns
    ///
    /// A `RefOrLiteral` instance containing the provided literal value, which can be used to represent a property value that is directly specified as a literal in the IR.
    pub fn from_literal(value: T) -> Self {
        Self::Literal(value)
    }

    /// Creates a new `RefOrLiteral` instance representing a reference to another token or value in the IR using a `JsonRef`
    ///
    /// # Arguments
    ///
    /// - `json_ref` - A `JsonRef` instance representing the reference to another token or value in the IR
    ///
    /// # Returns
    ///
    /// A `RefOrLiteral` instance containing the provided `JsonRefObject`, which can be used to represent a property value that references another token or value in the IR.
    pub fn from_ref(json_ref: JsonRefObject) -> Self {
        Self::Ref(json_ref)
    }

    /// Checks if the `RefOr` instance contains a literal value of type `T`
    ///
    /// # Returns
    ///
    /// `true` if the `RefOrLiteral` instance is a `Literal`, `false` if it is a `Ref`.
    pub fn is_literal(&self) -> bool {
        matches!(self, Self::Literal(_))
    }

    /// Checks if the `RefOrLiteral` instance contains a reference to another token or value in the IR using a `JsonRef`
    ///
    /// # Returns
    ///
    /// `true` if the `RefOrLiteral` instance is a `Ref`, `false` if it is a `Literal`.
    pub fn is_ref(&self) -> bool {
        matches!(self, Self::Ref(_))
    }

    /// Unwraps the `RefOrLiteral` instance and returns a reference to the literal value of type `T` if it is a `Literal`, or `None` if it is a `Ref`
    ///
    /// # Returns
    ///
    /// An `Option<&T>` which will contain a reference to the literal value of type `T` if the `RefOrLiteral` instance is a `Literal`, or `None` if it is a
    /// `Ref`, indicating that the value is a reference to another token or value in the IR rather than a literal value.
    pub fn as_literal(&self) -> Option<&T> {
        if let Self::Literal(value) = self {
            Some(value)
        } else {
            None
        }
    }
}

impl<'a, T: TryFromJson<'a>> TryFromJson<'a> for RefOrLiteral<T> {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        // First, we will try to parse the value as a reference using the JsonRef parser
        match JsonRefObject::try_from_json(ctx, path, value) {
            ParseState::Parsed(json_ref) => return ParseState::Parsed(Self::from_ref(json_ref)),
            ParseState::Invalid => return ParseState::Invalid,
            ParseState::NoMatch => {
                // If it does not match the JsonRef parser, we will try to parse it as a literal value of type T
                match T::try_from_json(ctx, path, value) {
                    ParseState::Parsed(literal) => ParseState::Parsed(Self::from_literal(literal)),
                    ParseState::Invalid => ParseState::Invalid,
                    ParseState::NoMatch => ParseState::NoMatch,
                }
            }
        }
    }
}

pub enum RefAliasOrLiteral<T> {
    Alias(TokenAlias),
    Ref(JsonRefObject),
    Literal(T),
}

impl<'a, T: TryFromJson<'a>> TryFromJson<'a> for RefAliasOrLiteral<T> {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        // First, attempt the value as a TokenAlias
        match TokenAlias::try_from_json(ctx, path, value) {
            ParseState::Parsed(alias) => return ParseState::Parsed(Self::Alias(alias)),
            ParseState::Invalid => return ParseState::Invalid,
            ParseState::NoMatch => {
                // If it does not match the TokenAlias parser, we will try to parse it as a reference using the JsonRefObject parser
                match JsonRefObject::try_from_json(ctx, path, value) {
                    ParseState::Parsed(json_ref) => return ParseState::Parsed(Self::Ref(json_ref)),
                    ParseState::Invalid => return ParseState::Invalid,
                    ParseState::NoMatch => {
                        // If it does not match the JsonRef parser, we will try to parse it as a literal value of type T
                        match T::try_from_json(ctx, path, value) {
                            ParseState::Parsed(literal) => {
                                ParseState::Parsed(Self::Literal(literal))
                            }
                            // The caller will likely want to error if Invalid or NoMatch is returned from the literal parser,
                            // since that means the value is not a valid literal and also not a valid reference or alias
                            ParseState::Invalid => ParseState::Invalid,
                            ParseState::NoMatch => ParseState::Invalid,
                        }
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_json_ref_parse() {
        let ref_str = "#/group1/subgroupA/tokenX";
        let json_ref = JsonRef::parse(ref_str).expect("Failed to parse JSON reference");
        assert_eq!(json_ref.document, None);
        assert_eq!(
            json_ref.pointer,
            JsonPointer::from_segments(vec!["group1", "subgroupA", "tokenX"])
        );

        let ref_str_with_doc = "document.json#/group1/subgroupA/tokenX";
        let json_ref_with_doc =
            JsonRef::parse(ref_str_with_doc).expect("Failed to parse JSON reference with document");
        assert_eq!(
            json_ref_with_doc.document,
            Some("document.json".to_string())
        );
        assert_eq!(
            json_ref_with_doc.pointer,
            JsonPointer::from_segments(vec!["group1", "subgroupA", "tokenX"])
        );

        let ref_str_without_pointer = "document.json";
        let json_ref_without_pointer = JsonRef::parse(ref_str_without_pointer)
            .expect("Failed to parse JSON reference without pointer");
        assert_eq!(
            json_ref_without_pointer.document,
            Some("document.json".to_string())
        );
        assert_eq!(json_ref_without_pointer.pointer, JsonPointer::new());

        let invalid_ref_str = "invalid_ref";
        let invalid_json_ref = JsonRef::parse(invalid_ref_str);
        assert!(
            invalid_json_ref.is_some(),
            "Expected to parse reference to another document without pointer, but got None"
        );
        let invalid_json_ref = JsonRef::parse("invalid#ref#string");
        assert!(
            invalid_json_ref.is_none(),
            "Expected to fail parsing invalid reference string with multiple '#' characters, but got Some"
        );
    }
}
