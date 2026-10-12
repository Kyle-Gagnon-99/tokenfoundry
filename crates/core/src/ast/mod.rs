//! The `ast` module provides the abstract syntax tree (AST) representation of the authored DTCG. This is used to represent the author's intent of DTCG
//! in a structured way, before being validated, resolved, and transformed into Intermediate Representation (IR) for further processing.
//!
//! Some validation does occur at the AST level. This would include things like ensuring that the syntax is correct, the types are valid, the structs
//! are valid, that there are no ambiguous objects. The AST is not fully validated to ensure that references are valid, all tokens are defined, and
//! more.

pub(crate) mod document;
pub(crate) mod group;
pub(crate) mod node;
pub(crate) mod reference;
pub(crate) mod span;
pub(crate) mod types;
pub(crate) mod value;
