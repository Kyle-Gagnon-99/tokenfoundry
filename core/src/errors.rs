//! The `errors` module defines custom error types for the token shift library,
//! which can be used to represent various error conditions that may occur during
//! token parsing, transformation, and generation. These error types can provide
//! more specific and informative error messages to help users understand and resolve issues with their tokens.

#[derive(Debug, Clone, Copy, serde::Serialize)]
pub enum Severity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, Copy, PartialEq, serde::Serialize)]
pub enum DiagnosticCode {
    MissingRequiredProperty,
    InvalidPropertyType,
    InvalidEnumValue,
    DuplicatePath,
    UnresolvedReference,
    InvalidReferenceTarget,
    CircularReference,
    UnsupportedFormat,
    InvalidTokenValue,
    InvalidTokenName,
    InvalidTokenPath,
    InvalidTokenType,
    InvalidTokenShape,
    InvalidGroupShape,
    InvalidPropertyValue,
    ResolverConflict,
    Other,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct Diagnostic {
    pub severity: Severity,
    pub code: DiagnosticCode,
    pub message: String,
    pub file_path: Option<String>,
    pub path: String,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct FatalError {
    pub message: String,
}
