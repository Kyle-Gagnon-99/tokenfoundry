//! The `reference` module contains the defintions and logic for referencing and aliasing

use regex::Regex;

use crate::ir::TokenPath;

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
    /// if let Some(alias) = TokenAlias::from_dtcg_alias(raw_value) {
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
    /// - `segments` - An iterator of items that can be converted into strings, representing the segments of the pointer, ordered from the root of the JSON document to the specific value
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

    /// Checks if a given string is a valid JSON Pointer according to the JSON Pointer specification (RFC 6901)
    ///
    /// Supports both normal JSON pointers and URI fragment identifiers (which start with a '#' character followed by a JSON pointer)
    ///
    /// # Arguments
    ///
    /// - `s` - The string to be checked for validity as a JSON Pointer
    ///
    /// # Returns
    ///
    /// `true` if the input string is a valid JSON Pointer according to the specification, `false` otherwise.
    pub fn is_valid_local_json_pointer(s: &str) -> bool {
        // Create a regular expression to match valid JSON Pointers according to RFC 6901
        // A valid JSON Pointer is either an empty string or a string that starts with a '/'
        // followed by zero or more segments, where each segment can contain any characters except for '~' and '/'
        // Additionally, we also want to support URI fragment identifiers, which start with a '#' character followed by a JSON Pointer
        let re = Regex::new(r"^(#)?(/([^/~]|~0|~1)*)*$").unwrap();
        re.is_match(s)
    }
}

impl From<&str> for JsonPointer {
    fn from(value: &str) -> Self {
        // First, we split the input string by the '/' character to get the individual segments of the JSON Pointer
        let segments = value
            .split('/')
            // We filter out any empty segments that may result from leading or trailing '/' characters
            .filter(|segment| !segment.is_empty())
            // We unescape any escaped characters in the segments according to the JSON Pointer specification
            .map(|segment| segment.replace("~1", "/").replace("~0", "~"))
            // We collect the resulting segments into a vector of strings
            .collect();
        Self { segments }
    }
}

impl From<String> for JsonPointer {
    fn from(value: String) -> Self {
        Self::from(value.as_str())
    }
}

/// The `JsonRefKind` enum represents the different kinds of JSON references that can be used in the IR to reference other tokens or values
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum JsonRefKind {
    LocalPointer {
        pointer: JsonPointer,
    },
    RelativeFile {
        file: String,
        pointer: Option<JsonPointer>,
    },
    AbsoluteFile {
        file: String,
        pointer: Option<JsonPointer>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JsonRef {
    pub raw_value: String,
    pub kind: JsonRefKind,
}

impl JsonRef {
    pub fn new(raw_value: String, kind: JsonRefKind) -> Self {
        Self { raw_value, kind }
    }

    pub fn new_local_pointer(raw_value: String, pointer: JsonPointer) -> Self {
        Self {
            raw_value,
            kind: JsonRefKind::LocalPointer { pointer },
        }
    }
}

/// The `RefOr` enum represents a property value that can either be a literal value of type `T` or a reference
/// to another token or value in the IR using a `JsonRef`.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RefOr<T> {
    Literal(T),
    Ref(JsonRef),
}

impl<T> RefOr<T> {
    /// Creates a new `RefOr` instance representing a literal value of type `T`
    ///
    /// # Arguments
    ///
    /// - `value` - The literal value of type `T` to be wrapped in the `RefOr` enum
    ///
    /// # Returns
    ///
    /// A `RefOr` instance containing the provided literal value, which can be used to represent a property value that is directly specified as a literal in the IR.
    pub fn from_literal(value: T) -> Self {
        RefOr::Literal(value)
    }

    /// Creates a new `RefOr` instance representing a reference to another token or value in the IR using a `JsonRef`
    ///
    /// # Arguments
    ///
    /// - `json_ref` - A `JsonRef` instance representing the reference to another token or value in the IR
    ///
    /// # Returns
    ///
    /// A `RefOr` instance containing the provided `JsonRef`, which can be used to represent a property value that references another token or value in the IR.
    pub fn from_ref(json_ref: JsonRef) -> Self {
        RefOr::Ref(json_ref)
    }
}
