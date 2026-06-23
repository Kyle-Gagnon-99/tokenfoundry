//! token-shift-core is a library that provides core functionality for the token-shift project.
//! token-shift is a project that aims to convert DTCG (Design Token Community Group) tokens
//! to various formats, such as CSS, Uniwind, Tailwind v4, Flutter, and more. The core library will contain the logic
//! for parsing, transforming, and generating tokens in different formats, while the CLI will provide a command-line interface
//! for users to interact with the core functionality.

pub mod analysis;
pub mod config;
pub mod errors;
pub mod input;
pub mod ir;
pub mod output;
pub mod parsing;
pub mod pipeline;
pub mod resolved;

pub const PROVENANCE_EXTENSION_KEY: &str = "tokenfoundry.dev/provenance";

#[derive(Debug, Clone)]
pub struct DiagnosticProvenance {
    pub source: String,
    pub pointer: String,
}
