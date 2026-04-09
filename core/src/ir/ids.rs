//! The `ids` module contains the definitions for various identifier types used within the library.

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

/// The `DocumentId` struct represents a unique identifier for a document in the IR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DocumentId(pub u64);

/// The `DocumentIdGenerator` struct is responsible for generating unique `DocumentId` values for documents in the IR
pub struct DocumentIdGenerator {
    next_id: u64,
}

impl DocumentIdGenerator {
    /// Creates a new `DocumentIdGenerator` with the initial `next_id` set to 1
    ///
    /// # Returns
    ///
    /// A new instance of `DocumentIdGenerator` ready to generate unique `DocumentId` values
    pub fn new() -> Self {
        Self { next_id: 1 }
    }

    /// Generates a new unique `DocumentId` by incrementing the `next_id` and returning the previous value as a `DocumentId`
    ///
    /// # Returns
    ///
    /// A `DocumentId` struct containing the unique identifier for a document in the IR
    pub fn generate(&mut self) -> DocumentId {
        let id = self.next_id;
        self.next_id += 1;
        DocumentId(id)
    }
}
