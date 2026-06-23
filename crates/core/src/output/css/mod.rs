//! The `css` module in output is responsible for generating CSS code from the internal representation of design tokens.
//! It takes the canonical token graph and produces CSS variables defined for the design system.

pub mod ast;
pub mod context;
pub mod conversion;
pub mod document;
pub mod utils;
