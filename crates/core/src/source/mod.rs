//! The `source` module provides functionality, utilities, and normalization for handling source files

pub(crate) mod map;
pub(crate) mod span;

// Re-export the contents of the `map` module for external use
pub use map::*;
pub use span::*;
