//! The `code` module defines custom error codes for the token shift library.

use std::fmt::Display;

use crate::errors::Severity;

/// A numeric code that uniquely identifies a specific diagnostic message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum DiagnosticCode {
    // 0xxx: JSON errors
    InvalidJsonSyntax = 1,
    DuplicateKeyInJson = 2,
    // 1xxx: DTCG shape errors
    // 2xxx: Reference errors
    // 3xxx: Token value validation errors
    // 4xxx: Extends / merging errors
    InvalidMergeRoot = 4001,
    GroupTokenConflict = 4002,
    // 5xxx: Resolver errors
    // 6xxx: Output errors
    // 7xxx: Configuration / CLI errors
    // 8xxx: General errors
    FileNotFound = 8001,
    FileReadError = 8002,
    FileWriteError = 8003,
    NotAFile = 8004,
    // 9xxx: Internal errors
    InternalError = 9001,
}

/// Categorizes the type of diagnostic message, allowing for better organization and filtering of diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum DiagnosticCategory {
    /// Source file / JSON parsing errors, such as invalid syntax or duplicate keys.
    Source,
    /// DTCG shape errors, such as invalid structure or missing required fields.
    DtcgShape,
    /// Reference errors, such as unresolved references or circular dependencies.
    Reference,
    /// Token value validation errors, such as invalid token values or unsupported formats.
    TokenValueValidation,
    /// Extends / merging errors, such as invalid extends or merging conflicts.
    ExtendsMerging,
    /// Resolver errors, such as invalid resolver configuration or failed resolution.
    Resolver,
    /// Output errors, such as failed output generation or invalid output format.
    Output,
    /// Configuration / CLI errors, such as invalid configuration or command-line arguments.
    Configuration,
    /// General errors that do not fit into any other category, such as unexpected conditions or unhandled cases.
    General,
    /// Internal errors that indicate a bug or unexpected condition in the compiler itself, such as unhandled cases or invalid state.
    Internal,
}

impl Display for DiagnosticCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "TF{:04}", self.number())
    }
}

/// Represents additional information about a diagnostic code, such as its severity and category, doc slugs, and any other relevant metadata
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DiagnosticCodeInfo {
    /// The default severity level of the diagnostic code, which can be used to determine how the diagnostic should be treated.
    pub default_severity: Severity,
    /// The category of the diagnostic code, which can be used for filtering or grouping diagnostics.
    pub category: DiagnosticCategory,
    /// A short, human-readable title for the diagnostic code, which can be used for display purposes.
    pub title: &'static str,
    /// The slug of the documentation page that provides more information about the diagnostic code, which can be used for linking to relevant documentation.
    pub doc_slug: &'static str,
}

impl DiagnosticCode {
    /// The numeric value of the diagnostic code, which can be used for comparison or sorting.
    pub fn number(self) -> u32 {
        self as u32
    }

    /// Returns a string representation of the diagnostic code, which can be used for display purposes.
    pub fn display_string(self) -> String {
        format!("{}", self)
    }

    /// Returns the category of the diagnostic code, which can be used for filtering or grouping diagnostics.
    pub fn category(self) -> DiagnosticCategory {
        match self.number() {
            0..=999 => DiagnosticCategory::Source,
            1000..=1999 => DiagnosticCategory::DtcgShape,
            2000..=2999 => DiagnosticCategory::Reference,
            3000..=3999 => DiagnosticCategory::TokenValueValidation,
            4000..=4999 => DiagnosticCategory::ExtendsMerging,
            5000..=5999 => DiagnosticCategory::Resolver,
            6000..=6999 => DiagnosticCategory::Output,
            7000..=7999 => DiagnosticCategory::Configuration,
            8000..=8999 => DiagnosticCategory::General,
            _ => DiagnosticCategory::Internal,
        }
    }

    /// A mapping of diagnostic codes to their default severity levels, which can be used to determine how the diagnostic should be treated.
    /// Note that the mapping is for default severity levels, and individual diagnostics may have different severity levels based on context or user configuration.
    pub fn default_severity(self) -> Severity {
        match self {
            Self::InvalidJsonSyntax => Severity::Error,
            Self::DuplicateKeyInJson => Severity::Error,
            Self::GroupTokenConflict => Severity::Warning,
            Self::InternalError => Severity::Error,
            Self::FileNotFound => Severity::Error,
            Self::FileReadError => Severity::Error,
            Self::FileWriteError => Severity::Error,
            Self::NotAFile => Severity::Error,
            _ => Severity::Error,
        }
    }

    /// Returns additional information about the diagnostic code, such as its severity and category, doc slugs, and any other relevant metadata.
    pub fn info(self) -> DiagnosticCodeInfo {
        // Set the default severity level for the diagnostic code based on its type. This can be used to determine how the diagnostic should be treated.
        let default_severity = self.default_severity();

        match self {
            Self::InvalidJsonSyntax => DiagnosticCodeInfo {
                default_severity,
                category: self.category(),
                title: "Invalid JSON Syntax",
                doc_slug: "invalid-json-syntax",
            },
            Self::DuplicateKeyInJson => DiagnosticCodeInfo {
                default_severity,
                category: self.category(),
                title: "Duplicate Key in JSON",
                doc_slug: "duplicate-key-in-json",
            },
            Self::InvalidMergeRoot => DiagnosticCodeInfo {
                default_severity,
                category: self.category(),
                title: "Invalid Merge Root",
                doc_slug: "invalid-merge-root",
            },
            Self::GroupTokenConflict => DiagnosticCodeInfo {
                default_severity,
                category: self.category(),
                title: "Group and Token Conflict",
                doc_slug: "group-token-conflict",
            },
            Self::InternalError => DiagnosticCodeInfo {
                default_severity,
                category: self.category(),
                title: "Internal Error",
                doc_slug: "internal-error",
            },
            Self::FileNotFound => DiagnosticCodeInfo {
                default_severity,
                category: self.category(),
                title: "File Not Found",
                doc_slug: "file-not-found",
            },
            Self::FileReadError => DiagnosticCodeInfo {
                default_severity,
                category: self.category(),
                title: "File Read Error",
                doc_slug: "file-read-error",
            },
            Self::FileWriteError => DiagnosticCodeInfo {
                default_severity,
                category: self.category(),
                title: "File Write Error",
                doc_slug: "file-write-error",
            },
            Self::NotAFile => DiagnosticCodeInfo {
                default_severity,
                category: self.category(),
                title: "Is Directory Error",
                doc_slug: "is-directory-error",
            },
            _ => DiagnosticCodeInfo {
                default_severity,
                category: self.category(),
                title: "Unknown Diagnostic Code",
                doc_slug: "unknown-diagnostic-code",
            },
        }
    }
}
