//! Output lifecycle entry points.
//!
//! Each emitter module will own target-specific generation logic.

pub mod android;
pub mod css;
pub mod lower;
pub mod tailwind;

pub use lower::*;
