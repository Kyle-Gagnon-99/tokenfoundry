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
    InvalidReference,
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

#[derive(Debug, Clone, serde::Serialize, thiserror::Error)]
#[error("{message}")]
pub struct FatalError {
    pub message: String,
}

#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("failed to read file '{path}': {source}")]
    FileRead {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("failed to parse JSON from '{path}': {source}")]
    JsonParse {
        path: String,
        #[source]
        source: serde_json::Error,
    },
    #[error(transparent)]
    Fatal(#[from] FatalError),
}
