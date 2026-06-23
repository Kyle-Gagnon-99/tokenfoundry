//! Analysis lifecycle entry points.
//!
//! This module groups post-parse analysis phases used by the compiler pipeline:
//! - Graph construction and validation
//! - Canonicalization for set operations
//! - Optimization metadata and transforms
//! - Invariant discovery

pub mod canonical;
pub mod graph;
pub mod invariants;
pub mod optimize;

pub use graph::*;
