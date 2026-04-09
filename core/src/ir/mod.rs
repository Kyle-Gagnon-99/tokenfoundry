//! The `ir` module contains the intermediate representation (IR) of design tokens that are used within the library.
//! The IR serves as a structured format for representing design tokens after they have been parsed from various input formats.

pub mod common;
pub mod ids;
pub mod node;
pub mod path;
pub mod reference;
pub mod utility;

pub use common::*;
pub use ids::*;
pub use node::*;
pub use path::*;
pub use reference::*;
pub use utility::*;
