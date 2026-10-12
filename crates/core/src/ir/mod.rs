//! The `ir` module in `core` provides the intermediate representation (IR) for the token-shift project.
//! This IR is a wrapper against the parsed JSON from the `json_spanned` crate. It provides additional metadata, used
//! for further processing and transformation within the token-shift project.
//! The IR serves as an intermediate layer between the raw JSON input and the final output formats, enabling more flexible and powerful transformations.

pub mod arena;
pub mod merge;
pub mod node;
