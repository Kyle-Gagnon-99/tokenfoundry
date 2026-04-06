//! The `ir` module defines the intermediate representation (IR) of design tokens and various data structures and types
//! relating to the IR.

use regex::Regex;

/// The `TokenId` struct represents a unique identifier for a design token in the IR
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TokenId(pub u64);

/// The `TokenIdGenerator` struct is responsible for generating unique `TokenId` values for design tokens in the IR
pub struct TokenIdGenerator {
    next_id: u64,
}

impl TokenIdGenerator {
    /// Creates a new `TokenIdGenerator` with the initial `next_id` set to 1
    pub fn new() -> Self {
        Self { next_id: 1 }
    }

    /// Generates a new unique `TokenId` by incrementing the `next_id` and returning the previous value as a `TokenId`
    ///
    /// # Returns
    ///
    /// A `TokenId` struct containing the unique identifier for a design token
    pub fn generate(&mut self) -> TokenId {
        let id = self.next_id;
        self.next_id += 1;
        TokenId(id)
    }
}

/// The `TokenPath` struct represents the hierarchical path of a design token in the IR, which is used to identify
/// the token and its position in the token hierarchy. The path is represented as a vector of strings, where each string
/// represents a segment of the path, such as a group name or a token name. The segments are ordered from the root of the hierarchy
/// to the specific token, allowing for easy traversal and organization of tokens in the IR.
/// This is directly derived from the layout of the token in the source JSON
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TokenPath {
    pub segments: Vec<String>,
}

impl TokenPath {
    /// Creates a new `TokenPath` with an empty vector of segments
    ///
    /// # Returns
    ///
    /// A `TokenPath` struct with an empty path, which can be used as a starting point for building the path of a design token in the IR
    pub fn new() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    /// Creates a new `TokenPath` from a vector of strings representing the segments of the path
    ///
    /// # Arguments
    ///
    /// * `segments` - A vector of strings representing the segments of the path, ordered from the root of the hierarchy to the specific token
    ///
    /// # Returns
    ///
    /// A `TokenPath` struct containing the provided segments, which can be used to identify a design token and its position in the token hierarchy in the IR
    pub fn from_segment_vec(segments: Vec<String>) -> Self {
        Self { segments }
    }

    /// Creates a new `TokenPath` from an iterator of items that can be converted into strings, representing the segments of the path
    ///
    /// # Arguments
    ///
    /// * `segments` - An iterator of items that can be converted into strings, representing the segments of the path, ordered from the root of the hierarchy to the specific token
    ///
    /// # Returns
    ///
    /// A `TokenPath` struct containing the provided segments, which can be used to identify a design token and its position in the token hierarchy in the IR
    ///
    /// # Examples
    ///
    /// ```
    /// use token_shift_core::token::ir::TokenPath;
    /// let path = TokenPath::from_segments(vec!["group1", "subgroupA", "tokenX"]);
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

    /// Creates a new `TokenPath` by appending a new segment to the existing path
    ///
    /// # Arguments
    ///
    /// * `segment` - A string representing the new segment to be appended to the existing path, which can be a group name or a token name
    ///
    /// # Returns
    ///
    /// A new `TokenPath` struct containing the existing segments with the new segment appended, which can be used to identify a
    /// design token and its position in the token hierarchy in the IR.
    ///
    /// # Examples
    ///
    /// ```
    /// use token_shift_core::token::ir::TokenPath;
    ///
    /// let base_path = TokenPath::from_segments(vec!["group1", "subgroupA"]);
    /// let new_path = base_path.child("tokenX");
    /// assert_eq!(new_path.segments, vec!["group1", "subgroupA", "tokenX"]);
    /// ```
    pub fn child(&self, segment: impl Into<String>) -> Self {
        let mut segments = self.segments.clone();
        segments.push(segment.into());
        Self { segments }
    }

    /// Converts the `TokenPath` into a dot-separated string representation, which can be used for display or referencing purposes
    ///
    /// # Returns
    ///
    /// A string representation of the `TokenPath`, where the segments are joined by dots (e.g., "group1.subgroupA.tokenX"), which can be used to identify a design token and its position in the token hierarchy in the IR.
    ///
    /// # Examples
    ///
    /// ```
    /// use token_shift_core::token::ir::TokenPath;
    /// let path = TokenPath::from_segments(vec!["group1", "subgroupA", "tokenX"]);
    /// assert_eq!(path.as_dot_path(), "group1.subgroupA.tokenX");
    /// ```
    pub fn as_dot_path(&self) -> String {
        self.segments.join(".")
    }
}

impl Into<String> for TokenPath {
    fn into(self) -> String {
        self.as_dot_path()
    }
}

/// The `DocumentId` struct represents a unique identifier for a document in the IR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DocumentId(pub u64);

pub struct DocumentIdGenerator {
    next_id: u64,
}

impl DocumentIdGenerator {
    pub fn new() -> Self {
        Self { next_id: 1 }
    }

    pub fn generate(&mut self) -> DocumentId {
        let id = self.next_id;
        self.next_id += 1;
        DocumentId(id)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TokenAlias {
    pub raw_value: String,
    pub target_path: TokenPath,
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
    /// use token_shift_core::token::ir::JsonPointer;
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
    pub fn is_valid_json_pointer(s: &str) -> bool {
        //
        true
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
    // Note: We are not currently going to support external URLs at the moment
    ExternalUrl {
        url: String,
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
    /// * `value` - The literal value of type `T` to be wrapped in the `RefOr` enum
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
    /// * `json_ref` - A `JsonRef` instance representing the reference to another token or value in the IR
    ///
    /// # Returns
    ///
    /// A `RefOr` instance containing the provided `JsonRef`, which can be used to represent a property value that references another token or value in the IR.
    pub fn from_ref(json_ref: JsonRef) -> Self {
        RefOr::Ref(json_ref)
    }
}

/// The `TokenValue` enum represents the source of a token's value in the IR, which can either be a literal value of type `T`,
/// an alias to another token or a reference to another token.
pub enum TokenValue<T> {
    Value(T),
    Alias(TokenAlias),
    Ref(JsonRef),
}

pub enum IrTokenType {}

//pub struct IrToken<T> {}
