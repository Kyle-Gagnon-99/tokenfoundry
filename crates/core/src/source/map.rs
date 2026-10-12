//! The `source` module provides functionality for handling source files

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::{
    diagnostics::{
        code::DiagnosticCode,
        diagnostic::{Diagnostic, DiagnosticSink},
    },
    ir::node::NodeId,
};

/// Identifies a source file by a unique numeric ID, which can be used to reference the source file in diagnostics and other parts of the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SourceId(u32);

impl SourceId {
    /// Returns the numeric value of the `SourceId`, which can be used for comparison or sorting.
    ///
    /// # Returns
    ///
    /// The numeric value of the `SourceId` as a `u32`.
    pub fn value(self) -> u32 {
        self.0
    }
}

#[derive(Debug, Default, Clone)]
pub struct SourceIdGenerator {
    next_id: u32,
}

impl SourceIdGenerator {
    /// Generates a new unique `SourceId` for a source file, incrementing the internal counter to ensure uniqueness.
    pub fn generate(&mut self) -> SourceId {
        let id = self.next_id;
        self.next_id += 1;
        SourceId(id)
    }
}

#[derive(Debug, Clone)]
pub struct SourceFile {
    pub id: SourceId,
    pub path: PathBuf,
    pub content: Arc<str>,
    pub root_node_id: NodeId,
}

impl SourceFile {
    /// Set the root node ID for the source file.
    ///
    /// # Parameters
    ///
    /// - `root_node_id`: The `NodeId` to set as the root node ID for the source file.
    pub fn set_root_node_id(&mut self, root_node_id: NodeId) {
        self.root_node_id = root_node_id;
    }
}

/// Contains the mapping information between the source file and the generated ID.
#[derive(Debug, Default, Clone)]
pub struct SourceMap {
    /// Maps a source file path to its corresponding `SourceId`
    files: HashMap<SourceId, SourceFile>,
    /// Maps a `SourceId` to its corresponding source file path
    id_to_path: HashMap<SourceId, PathBuf>,
    /// Maps a source file path to its corresponding `SourceId`
    path_to_id: HashMap<PathBuf, SourceId>,
    /// Generates unique IDs for source files, ensuring that each source file is assigned a distinct identifier.
    generator: SourceIdGenerator,
}

impl SourceMap {
    /// Creates a new `SourceMap` instance, initializing the internal mappings and the ID generator.
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers a source file path and returns its corresponding `SourceId`. If the path already exists, it returns the existing `SourceId`.
    ///
    /// # Errors
    ///
    /// Returns an error if the path does not exist or the file could not be read. In such cases, a diagnostic message is emitted to the provided
    /// `DiagnosticSink`.
    pub fn register<P: AsRef<Path>>(
        &mut self,
        path: P,
        diagnostic_sink: &mut impl DiagnosticSink,
    ) -> Result<SourceId, ()> {
        // First, get the path reference
        let path_ref = path.as_ref();

        // If the path already exists, return the existing SourceId
        if let Some(&id) = self.path_to_id.get(path_ref) {
            return Ok(id);
        }

        // Next, check if the path is valid and exists
        if !path_ref.exists() {
            diagnostic_sink.emit(Diagnostic::new(DiagnosticCode::FileNotFound).with_message(
                format!(
                    "{} does not exist.",
                    path_ref.to_str().unwrap_or("<invalid_path>")
                ),
            ));
            return Err(());
        }

        // Check if the path is a file
        if !path_ref.is_file() {
            diagnostic_sink.emit(
                Diagnostic::new(DiagnosticCode::NotAFile)
                    .with_message(format!(
                        "{} is not a file.",
                        path_ref.to_str().unwrap_or("<invalid_path>")
                    ))
                    .help("Provide a valid file path instead of a directory."),
            );
            return Err(());
        }

        // Check if the file can be read and if so, read its content. If not, emit a diagnostic and return an error.
        let content = match std::fs::read_to_string(path_ref) {
            Ok(content) => content,
            Err(_) => {
                diagnostic_sink.emit(
                    Diagnostic::new(DiagnosticCode::FileReadError)
                        .with_message(format!(
                            "Failed to read the file at {}.",
                            path_ref.to_str().unwrap_or("<invalid_path>")
                        ))
                        .help("Ensure the file is readable and not locked by another process."),
                );
                return Err(());
            }
        };

        // Generate a new SourceId for the source file and update the mappings
        let new_id = self.generator.generate();
        self.path_to_id.insert(path_ref.to_path_buf(), new_id);
        self.id_to_path.insert(new_id, path_ref.to_path_buf());

        // Finally, create a new SourceFile instance and insert it into the files mapping
        // Set the root node ID for the source file to u32::MAX as a placeholder. 0 is valid, so
        // we use u32::MAX to indicate an uninitialized root node ID.
        let source_file = SourceFile {
            id: new_id,
            path: path_ref.to_path_buf(),
            content: Arc::from(content),
            root_node_id: NodeId(u32::MAX),
        };
        self.files.insert(new_id, source_file);

        Ok(new_id)
    }

    pub fn get_path(&self, id: SourceId) -> Option<&Path> {
        self.id_to_path.get(&id).map(|p| p.as_path())
    }

    /// Clears all registered source files and resets the internal state of the `SourceMap`, including the ID generator. This effectively removes all mappings and allows for a fresh start.
    pub fn clear(&mut self) {
        self.files.clear();
        self.id_to_path.clear();
        self.path_to_id.clear();
        self.generator = SourceIdGenerator::default();
    }

    /// Retrieves the content of a source file by its `SourceId`. If the `SourceId` is valid and corresponds to a registered source file, it returns the content as an `Arc<str>`. If the `SourceId` is invalid or not found, it returns `None`.
    pub fn get_content(&self, id: SourceId) -> Option<Arc<str>> {
        self.files.get(&id).map(|file| file.content.clone())
    }
}
