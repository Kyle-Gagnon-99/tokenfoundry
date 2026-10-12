//! The `errors` module defines custom error types for the token shift library,
//! which can be used to represent various error conditions that may occur during
//! token parsing, transformation, and generation. These error types can provide
//! more specific and informative error messages to help users understand and resolve issues with their tokens.

/// Represents the severity level of a diagnostic message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Severity {
    Error,
    Warning,
    Info,
}
