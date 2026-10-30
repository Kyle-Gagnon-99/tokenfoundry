//! The `source` module provides functionality for handling source files

use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::Arc,
};

use crate::diagnostics::{
    code::DiagnosticCode,
    diagnostic::{Diagnostic, DiagnosticSink},
};

/// Identifies a source file by a unique numeric ID, which can be used to reference the source file in diagnostics and other parts of the system.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
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
    pub content: Arc<str>,
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

        Ok(self.register_source(path_ref, content))
    }

    /// Registers a source whose content the caller already has, such as from a browser, a Node host or a test, so
    /// nothing is read from disk. `path` is only a label for diagnostics and for finding the source again. If it is
    /// already registered, the existing `SourceId` is returned and the existing content is kept.
    pub fn register_source<P: AsRef<Path>>(
        &mut self,
        path: P,
        content: impl Into<Arc<str>>,
    ) -> SourceId {
        let path_ref = path.as_ref();
        if let Some(&id) = self.path_to_id.get(path_ref) {
            return id;
        }

        // Generate a new SourceId for the source file and update the mappings
        let new_id = self.generator.generate();
        self.path_to_id.insert(path_ref.to_path_buf(), new_id);
        self.id_to_path.insert(new_id, path_ref.to_path_buf());

        // Finally, create a new SourceFile instance and insert it into the files mapping
        // Set the root node ID for the source file to u32::MAX as a placeholder. 0 is valid, so
        // we use u32::MAX to indicate an uninitialized root node ID.
        let source_file = SourceFile {
            content: content.into(),
        };
        self.files.insert(new_id, source_file);

        new_id
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostics::diagnostic::DiagnosticStore;

    #[test]
    fn in_memory_sources_never_touch_the_disk() {
        let mut map = SourceMap::new();
        let id = map.register_source("virtual/tokens.json", r#"{"a": 1}"#);
        assert_eq!(&*map.get_content(id).unwrap(), r#"{"a": 1}"#);
        assert_eq!(map.get_path(id), Some(Path::new("virtual/tokens.json")));
    }

    #[test]
    fn registering_the_same_path_twice_keeps_the_first_content() {
        let mut map = SourceMap::new();
        let first = map.register_source("a.json", "{}");
        let second = map.register_source("a.json", "[]");
        assert_eq!(first, second);
        assert_eq!(&*map.get_content(first).unwrap(), "{}");
    }

    #[test]
    fn distinct_paths_get_distinct_ids() {
        let mut map = SourceMap::new();
        assert_ne!(
            map.register_source("a.json", "{}"),
            map.register_source("b.json", "{}")
        );
    }

    #[test]
    fn a_file_on_disk_is_read_through_the_same_path() {
        let path = std::env::temp_dir().join("tokenfoundry_register_test.json");
        std::fs::write(&path, "{\"x\": 2}").unwrap();
        let mut map = SourceMap::new();
        let mut diags = DiagnosticStore::new();
        let id = map.register(&path, &mut diags).unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!(&*map.get_content(id).unwrap(), "{\"x\": 2}");
        // A second registration returns the cached id.
        assert_eq!(map.register(&path, &mut diags), Ok(id));
        assert!(diags.diagnostics().is_empty());
    }

    #[test]
    fn a_missing_file_is_a_diagnostic() {
        let mut map = SourceMap::new();
        let mut diags = DiagnosticStore::new();
        assert!(
            map.register("/no/such/tokenfoundry.json", &mut diags)
                .is_err()
        );
        assert_eq!(diags.diagnostics().len(), 1);
    }
}
