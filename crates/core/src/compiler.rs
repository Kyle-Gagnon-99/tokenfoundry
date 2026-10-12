//! The `compiler` module provides the core functionality for compiling DTCG into various outputs. This module contains the main entry point
//! for the core library.

use std::path::Path;

use crate::{config::Config, diagnostics::diagnostic::DiagnosticStore, source::SourceMap};

#[derive(Debug)]
pub struct CompileSession<'a> {
    pub config: &'a Config,
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

    /// Compiles the DTCG into various outputs based on the provided configuration and resolver.
    ///
    /// # Arguments
    ///
    /// * `config`: The configuration options for the compilation process, which may include settings for log level, Tailwind CSS transformation, and other options.
    /// * `resolver`: A path to a resolver that will be used to locate and process the DTCG files during the compilation process.
    ///
    /// # Returns
    ///
    /// `Result<(), CompileFailed>` indicating the success or failure of the compilation process. If the compilation is successful, it returns `Ok(())`. If there is an error during the compilation, it returns `Err(CompileFailed)`.
    /// Use `get_diagnostics()` to retrieve the diagnostics for the compilation process, including errors.
    pub fn compile<P: AsRef<Path>>(
        &mut self,
        config: Config,
        resolver: P,
    ) -> Result<(), CompileFailed> {
        // Start a new compile session with the provided configuration, sources, and diagnostics
        self.sources.clear();
        self.diagnostics.clear();

        let mut session = CompileSession {
            config: &config,
            sources: &mut self.sources,
            diagnostics: &mut self.diagnostics,
        };

        Ok(())
    }
}
