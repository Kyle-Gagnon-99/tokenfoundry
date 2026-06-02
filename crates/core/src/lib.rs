//! token-shift-core is a library that provides core functionality for the token-shift project.
//! token-shift is a project that aims to convert DTCG (Design Token Community Group) tokens
//! to various formats, such as CSS, Uniwind, Tailwind v4, Flutter, and more. The core library will contain the logic
//! for parsing, transforming, and generating tokens in different formats, while the CLI will provide a command-line interface
//! for users to interact with the core functionality.

use crate::{
    config::Config,
    errors::{Diagnostic, DiagnosticCode},
    ir::{IrTokenType, JsonPointer},
};
use std::collections::HashMap;
use tracing::{error, info, warn};

pub mod commands;
pub mod config;
pub mod errors;
pub mod graph;
pub mod ir;
pub mod parser;
pub mod resolver;
pub mod token;

pub const PROVENANCE_EXTENSION_KEY: &str = "tokenfoundry.dev/provenance";

#[derive(Debug, Clone)]
pub struct DiagnosticProvenance {
    pub source: String,
    pub pointer: String,
}

#[derive(Debug, Clone)]
pub struct ParserContext {
    pub file_content: String,
    pub errors: Vec<errors::Diagnostic>,
    pub warnings: Vec<errors::Diagnostic>,
    pub infos: Vec<errors::Diagnostic>,
    pub current_path: JsonPointer,
    pub current_type: Option<IrTokenType>,
    pub diagnostic_provenance: HashMap<String, DiagnosticProvenance>,
}

impl ParserContext {
    pub fn new(file_content: String) -> Self {
        Self {
            file_content,
            errors: Vec::new(),
            warnings: Vec::new(),
            infos: Vec::new(),
            current_path: JsonPointer::new(),
            current_type: None,
            diagnostic_provenance: HashMap::new(),
        }
    }

    pub fn set_diagnostic_provenance(
        &mut self,
        diagnostic_provenance: HashMap<String, DiagnosticProvenance>,
    ) {
        self.diagnostic_provenance = diagnostic_provenance;
    }

    fn resolve_diagnostic_location(&self, path: &str) -> (Option<String>, String) {
        let segments: Vec<&str> = path
            .split('/')
            .filter(|segment| !segment.is_empty())
            .collect();

        for depth in (0..=segments.len()).rev() {
            let key = if depth == 0 {
                "/".to_string()
            } else {
                format!("/{}", segments[..depth].join("/"))
            };

            if let Some(provenance) = self.diagnostic_provenance.get(&key) {
                let remainder = if depth < segments.len() {
                    format!("/{}", segments[depth..].join("/"))
                } else {
                    String::new()
                };

                let diagnostic_path = if provenance.pointer == "#" {
                    if remainder.is_empty() {
                        "#".to_string()
                    } else {
                        format!("#{}", remainder)
                    }
                } else {
                    format!("{}{}", provenance.pointer, remainder)
                };

                return (Some(provenance.source.clone()), diagnostic_path);
            }
        }

        (None, path.to_string())
    }

    pub fn push_to_errors(
        &mut self,
        diagnostic: DiagnosticCode,
        message: impl Into<String>,
        path: String,
    ) {
        let message = message.into();
        let (diagnostic_file_path, diagnostic_path) = self.resolve_diagnostic_location(&path);
        error!(
            code = ?diagnostic,
            file_path = ?diagnostic_file_path,
            path = %diagnostic_path,
            message = %message,
            "parser diagnostic"
        );
        self.errors.push(Diagnostic {
            severity: errors::Severity::Error,
            code: diagnostic,
            message,
            file_path: diagnostic_file_path,
            path: diagnostic_path,
        });
    }

    pub fn push_to_warnings(
        &mut self,
        diagnostic: DiagnosticCode,
        message: impl Into<String>,
        path: String,
    ) {
        let message = message.into();
        let (diagnostic_file_path, diagnostic_path) = self.resolve_diagnostic_location(&path);
        warn!(
            code = ?diagnostic,
            file_path = ?diagnostic_file_path,
            path = %diagnostic_path,
            message = %message,
            "parser diagnostic"
        );
        self.warnings.push(Diagnostic {
            severity: errors::Severity::Warning,
            code: diagnostic,
            message,
            file_path: diagnostic_file_path,
            path: diagnostic_path,
        });
    }

    pub fn push_to_infos(
        &mut self,
        diagnostic: DiagnosticCode,
        message: impl Into<String>,
        path: String,
    ) {
        let message = message.into();
        let (diagnostic_file_path, diagnostic_path) = self.resolve_diagnostic_location(&path);
        info!(
            code = ?diagnostic,
            file_path = ?diagnostic_file_path,
            path = %diagnostic_path,
            message = %message,
            "parser diagnostic"
        );
        self.infos.push(Diagnostic {
            severity: errors::Severity::Info,
            code: diagnostic,
            message,
            file_path: diagnostic_file_path,
            path: diagnostic_path,
        });
    }

    pub fn get_current_path(&self) -> String {
        self.current_path.to_string()
    }

    pub fn push_to_current_path(&mut self, segment: String) {
        self.current_path.push(segment);
    }

    pub fn pop_current_path(&mut self) {
        self.current_path.segments.pop();
    }

    pub fn set_current_type(&mut self, token_type: IrTokenType) {
        self.current_type = Some(token_type);
    }

    pub fn get_current_type(&self) -> Option<IrTokenType> {
        self.current_type
    }
}

pub fn build_tokens(config: Config) {}

pub fn inspect_tokens(config: Config) {}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_context() -> ParserContext {
        ParserContext::new("{}".to_string())
    }

    #[test]
    fn diagnostics_use_longest_matching_provenance_path() {
        let mut ctx = make_context();

        let mut provenance = HashMap::new();
        provenance.insert(
            "/semantic".to_string(),
            DiagnosticProvenance {
                source: "semantic.tokens.json".to_string(),
                pointer: "#/semantic".to_string(),
            },
        );
        provenance.insert(
            "/semantic/background".to_string(),
            DiagnosticProvenance {
                source: "theme/dark.tokens.json".to_string(),
                pointer: "#/semantic/background".to_string(),
            },
        );
        ctx.set_diagnostic_provenance(provenance);

        ctx.push_to_errors(
            DiagnosticCode::InvalidPropertyValue,
            "boom",
            "/semantic/background/canvas/$value".to_string(),
        );

        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(
            ctx.errors[0].file_path.as_deref(),
            Some("theme/dark.tokens.json")
        );
        assert_eq!(ctx.errors[0].path, "#/semantic/background/canvas/$value");
    }

    #[test]
    fn diagnostics_fall_back_without_provenance() {
        let mut ctx = make_context();

        ctx.push_to_errors(
            DiagnosticCode::Other,
            "fallback",
            "/brand/primary/$value".to_string(),
        );

        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].file_path, None);
        assert_eq!(ctx.errors[0].path, "/brand/primary/$value");
    }

    #[test]
    fn diagnostics_map_root_pointer_with_remainder() {
        let mut ctx = make_context();

        let mut provenance = HashMap::new();
        provenance.insert(
            "/".to_string(),
            DiagnosticProvenance {
                source: "root-source.json".to_string(),
                pointer: "#".to_string(),
            },
        );
        ctx.set_diagnostic_provenance(provenance);

        ctx.push_to_errors(DiagnosticCode::Other, "root mapped", "/a/b/c".to_string());

        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].file_path.as_deref(), Some("root-source.json"));
        assert_eq!(ctx.errors[0].path, "#/a/b/c");
    }

    #[test]
    fn warnings_and_infos_also_use_provenance_mapping() {
        let mut ctx = make_context();

        let mut provenance = HashMap::new();
        provenance.insert(
            "/semantic/body".to_string(),
            DiagnosticProvenance {
                source: "theme/light.tokens.json".to_string(),
                pointer: "#/semantic/body".to_string(),
            },
        );
        ctx.set_diagnostic_provenance(provenance);

        ctx.push_to_warnings(
            DiagnosticCode::InvalidPropertyValue,
            "warn",
            "/semantic/body/$value/lineHeight".to_string(),
        );
        ctx.push_to_infos(
            DiagnosticCode::Other,
            "info",
            "/semantic/body/$value/fontWeight".to_string(),
        );

        assert_eq!(ctx.warnings.len(), 1);
        assert_eq!(ctx.infos.len(), 1);

        assert_eq!(
            ctx.warnings[0].file_path.as_deref(),
            Some("theme/light.tokens.json")
        );
        assert_eq!(ctx.warnings[0].path, "#/semantic/body/$value/lineHeight");

        assert_eq!(
            ctx.infos[0].file_path.as_deref(),
            Some("theme/light.tokens.json")
        );
        assert_eq!(ctx.infos[0].path, "#/semantic/body/$value/fontWeight");
    }
}
