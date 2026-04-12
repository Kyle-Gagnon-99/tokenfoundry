//! token-shift-core is a library that provides core functionality for the token-shift project.
//! token-shift is a project that aims to convert DTCG (Design Token Community Group) tokens
//! to various formats, such as CSS, Uniwind, Tailwind v4, Flutter, and more. The core library will contain the logic
//! for parsing, transforming, and generating tokens in different formats, while the CLI will provide a command-line interface
//! for users to interact with the core functionality.

use crate::{
    errors::{Diagnostic, DiagnosticCode},
    ir::IrDocument,
};

pub mod config;
pub mod errors;
pub mod ir;
pub mod parser;
pub mod token;

#[derive(Debug, Clone, serde::Serialize)]
pub enum FileFormat {
    Json,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct ParserContext {
    pub file_path: String,
    pub file_format: FileFormat,
    pub file_content: String,
    pub errors: Vec<errors::Diagnostic>,
    pub warnings: Vec<errors::Diagnostic>,
    pub infos: Vec<errors::Diagnostic>,
}

impl ParserContext {
    pub fn new(file_path: String, file_format: FileFormat, file_content: String) -> Self {
        Self {
            file_path,
            file_format,
            file_content,
            errors: Vec::new(),
            warnings: Vec::new(),
            infos: Vec::new(),
        }
    }

    pub fn push_to_errors(
        &mut self,
        diagnostic: DiagnosticCode,
        message: impl Into<String>,
        path: String,
    ) {
        self.errors.push(Diagnostic {
            severity: errors::Severity::Error,
            code: diagnostic,
            message: message.into(),
            file_path: Some(self.file_path.clone()),
            path,
        });
    }

    pub fn push_to_warnings(
        &mut self,
        diagnostic: DiagnosticCode,
        message: impl Into<String>,
        path: String,
    ) {
        self.warnings.push(Diagnostic {
            severity: errors::Severity::Warning,
            code: diagnostic,
            message: message.into(),
            file_path: Some(self.file_path.clone()),
            path,
        });
    }

    pub fn push_to_infos(
        &mut self,
        diagnostic: DiagnosticCode,
        message: impl Into<String>,
        path: String,
    ) {
        self.infos.push(Diagnostic {
            severity: errors::Severity::Info,
            code: diagnostic,
            message: message.into(),
            file_path: Some(self.file_path.clone()),
            path,
        });
    }
}
