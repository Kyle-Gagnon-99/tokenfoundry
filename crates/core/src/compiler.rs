//! The `compiler` module provides the core functionality for compiling DTCG into various outputs. This module contains the main entry point
//! for the core library.

use crate::{
    ast::resolve::ResolvedTokens,
    diagnostics::diagnostic::DiagnosticStore,
    resolver::{Bundle, Input, SourceProvider},
    source::SourceMap,
};

#[derive(Debug)]
pub struct CompileSession<'a> {
    pub sources: &'a mut SourceMap,
    pub diagnostics: &'a mut DiagnosticStore,
}

#[derive(Debug, Clone)]
pub struct CompileFailed;

#[derive(Debug, Clone)]
pub struct DtcgCompiler {
    sources: SourceMap,
    diagnostics: DiagnosticStore,
}

impl DtcgCompiler {
    /// Initializes the DTCG compiler with a new state
    pub fn new() -> Self {
        Self {
            sources: SourceMap::new(),
            diagnostics: DiagnosticStore::new(),
        }
    }

    /// Compiles a resolver for one input into fully resolved tokens. Nothing here touches the filesystem: the
    /// caller gives the resolver's text and a provider for the files it references.
    ///
    /// # Arguments
    ///
    /// * `config`: The configuration options for the compilation process.
    /// * `resolver_path`: The resolver's path. Relative file references are joined to its directory, and it is used as the label in diagnostics.
    /// * `resolver_text`: The contents of the resolver document.
    /// * `provider`: Supplies the contents of every file the resolver references.
    /// * `input`: The context to use for each modifier. Modifiers with a default may be left out.
    ///
    /// # Returns
    ///
    /// The resolved tokens, or `CompileFailed` if any error was reported. Use [`DtcgCompiler::diagnostics`] to see
    /// everything that was reported, including warnings on success.
    pub fn compile(
        &mut self,
        resolver_path: &str,
        resolver_text: &str,
        provider: &dyn SourceProvider,
        input: &Input,
    ) -> Result<ResolvedTokens, CompileFailed> {
        self.sources.clear();
        self.diagnostics.clear();

        let session = CompileSession {
            sources: &mut self.sources,
            diagnostics: &mut self.diagnostics,
        };

        let result = Bundle::load(
            resolver_path,
            resolver_text,
            provider,
            session.sources,
            session.diagnostics,
        )
        .and_then(|bundle| bundle.compile(input, session.diagnostics))
        .map(|compiled| compiled.resolved());

        match result {
            Some(tokens) if !self.diagnostics.has_errors() => Ok(tokens),
            _ => Err(CompileFailed),
        }
    }

    pub fn diagnostics(&self) -> &DiagnosticStore {
        &self.diagnostics
    }

    pub fn sources(&self) -> &SourceMap {
        &self.sources
    }
}

impl Default for DtcgCompiler {
    fn default() -> Self {
        Self::new()
    }
}
