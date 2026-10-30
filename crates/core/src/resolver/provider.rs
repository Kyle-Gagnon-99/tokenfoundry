//! How the resolver gets file contents. `core` never touches the disk: the host (CLI, Node, WASM, Python, tests)
//! supplies the text, either on demand through a [`SourceProvider`] or up front through [`MemorySources`].
//!
//! Paths are plain `/`-separated strings, joined and normalized lexically, so behavior is identical on every
//! platform and in WASM.

use std::collections::HashMap;

/// Why a host could not supply a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProviderError {
    NotFound,
    Unreadable(String),
}

/// Supplies the contents of files referenced by a resolver. Paths are already joined to the referencing document
/// and normalized.
pub trait SourceProvider {
    fn read(&self, path: &str) -> Result<String, ProviderError>;
}

/// A provider backed by a map, for hosts that already hold the contents.
#[derive(Debug, Default, Clone)]
pub struct MemorySources {
    files: HashMap<String, String>,
}

impl MemorySources {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn insert(&mut self, path: &str, content: impl Into<String>) -> &mut Self {
        self.files.insert(normalize(path), content.into());
        self
    }
}

impl SourceProvider for MemorySources {
    fn read(&self, path: &str) -> Result<String, ProviderError> {
        self.files
            .get(&normalize(path))
            .cloned()
            .ok_or(ProviderError::NotFound)
    }
}

/// Resolves `relative` against the directory of the document at `base`.
pub fn join(base: &str, relative: &str) -> String {
    let dir = base.rfind('/').map_or("", |i| &base[..i]);
    if dir.is_empty() {
        normalize(relative)
    } else {
        normalize(&format!("{dir}/{relative}"))
    }
}

/// Drops `.` and empty segments and applies `..`. Leading `..` that cannot be applied are kept, so a resolver in a
/// subdirectory can still reach a sibling directory.
pub fn normalize(path: &str) -> String {
    let mut out: Vec<&str> = Vec::new();
    for seg in path.split('/') {
        match seg {
            "" | "." => {}
            ".." => {
                if matches!(out.last(), Some(last) if *last != "..") {
                    out.pop();
                } else {
                    out.push("..");
                }
            }
            s => out.push(s),
        }
    }
    out.join("/")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn join_resolves_against_the_document_directory() {
        assert_eq!(
            join("tokens/resolver.json", "./base.json"),
            "tokens/base.json"
        );
        assert_eq!(join("tokens/resolver.json", "../x/a.json"), "x/a.json");
        assert_eq!(join("resolver.json", "a/b.json"), "a/b.json");
    }

    #[test]
    fn normalize_keeps_unresolvable_parent_segments() {
        assert_eq!(normalize("../a/./b//c.json"), "../a/b/c.json");
        assert_eq!(normalize("a/../../b"), "../b");
    }

    #[test]
    fn memory_sources_match_after_normalization() {
        let mut m = MemorySources::new();
        m.insert("./a/b.json", "{}");
        assert_eq!(m.read("a/x/../b.json").unwrap(), "{}");
        assert_eq!(m.read("c.json"), Err(ProviderError::NotFound));
    }
}
