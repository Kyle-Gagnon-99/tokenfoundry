//! The `path` module defines the `TokenPath` struct, which represents the hierarchical path of a design token in the IR. The `TokenPath` is used to uniquely identify tokens and their relationships within the design system.

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
    /// - `segments` - A vector of strings representing the segments of the path, ordered from the root of the hierarchy to the specific token
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
    /// - `segments` - An iterator of items that can be converted into strings, representing the segments of the path, ordered from the root of the hierarchy to the specific token
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
    /// - `segment` - A string representing the new segment to be appended to the existing path, which can be a group name or a token name
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
