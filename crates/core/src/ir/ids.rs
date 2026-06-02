//! The `ids` module contains identifier types used within the IR.
//! At this stage, IDs are token-scoped rather than document-scoped.

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
