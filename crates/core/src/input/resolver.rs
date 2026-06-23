//! The `resolver` module is responsible for parsing and understanding the resolver configuration files.
//!
//! This module provides functionality to:
//! - Parse resolver JSON documents
//! - Validate resolver structure and inputs
//! - Execute the resolution process (input validation, ordering, aliasing)
//! - Generate final token sets based on selected contexts

use crate::{
    PROVENANCE_EXTENSION_KEY, ParserContext,
    analysis::graph::TokenGraph,
    errors::Diagnostic,
    ir::{JsonPointerRef, ResolutionInput, ResolverDocument, TokenAlias},
    parsing::parse_document,
};
use serde_json::{Value, json};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub const RESOLVER_VERSION: &str = "2025.10";

/// Errors that can occur during resolver operations
#[derive(Debug, thiserror::Error)]
pub enum ResolverError {
    /// Invalid resolver document structure
    #[error("failed to parse resolver document JSON: {source}")]
    InvalidDocument {
        #[source]
        source: serde_json::Error,
    },
    /// Version mismatch (must be 2025.10)
    #[error("invalid resolver version: expected {expected}, got {found}")]
    InvalidVersion {
        expected: &'static str,
        found: String,
    },
    /// Resolution order is required
    #[error("missing required 'resolutionOrder' in resolver document")]
    MissingResolutionOrder,
    /// Invalid reference object
    #[error("invalid reference: {message}")]
    InvalidReference { message: String },
    /// Circular reference detected
    #[error("circular reference detected: {message}")]
    CircularReference { message: String },
    /// Invalid input for resolution
    #[error("invalid resolution input: {errors:?}")]
    InvalidInput { errors: Vec<String> },
    /// Failed to read resolver or external JSON file
    #[error("failed to read file '{path}': {source}")]
    ReadFile {
        path: String,
        #[source]
        source: std::io::Error,
    },
    /// Failed to parse external JSON file
    #[error("failed to parse JSON file '{path}': {source}")]
    ParseExternalJson {
        path: String,
        #[source]
        source: serde_json::Error,
    },
    /// Invalid JSON pointer
    #[error("invalid JSON pointer '{pointer}': {message}")]
    InvalidJsonPointer { pointer: String, message: String },
}

/// Resolver configuration and operations
pub struct Resolver {
    document: ResolverDocument,
    /// Base directory for resolving relative file paths
    base_path: PathBuf,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResolveOptions {
    pub materialize_aliases: bool,
    pub materialize_property_refs: bool,
    pub convert_token_refs_to_aliases: bool,
}

#[derive(Debug, Clone)]
pub struct ResolvedGraphOutput {
    pub resolved_tokens: Value,
    pub document: crate::ir::IrDocument,
    pub graph: TokenGraph,
    pub parser_diagnostics: Vec<Diagnostic>,
    pub graph_diagnostics: Vec<Diagnostic>,
}

impl Default for ResolveOptions {
    fn default() -> Self {
        Self {
            materialize_aliases: true,
            materialize_property_refs: true,
            convert_token_refs_to_aliases: true,
        }
    }
}

impl Resolver {
    /// Create a new resolver from a JSON string
    /// Uses current directory as the base path for relative file references
    pub fn from_json(json_str: &str) -> Result<Self, ResolverError> {
        Self::from_json_with_base(
            json_str,
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        )
    }

    /// Create a new resolver from a JSON string with a specific base path for file resolution
    /// The base_path should be the directory containing the resolver document
    pub fn from_json_with_base<P: AsRef<Path>>(
        json_str: &str,
        base_path: P,
    ) -> Result<Self, ResolverError> {
        let document: ResolverDocument = serde_json::from_str(json_str)
            .map_err(|source| ResolverError::InvalidDocument { source })?;

        Self::validate_document(&document)?;
        Ok(Resolver {
            document,
            base_path: base_path.as_ref().to_path_buf(),
        })
    }

    /// Load a resolver from a file path
    /// Automatically uses the file's directory as the base path for relative references
    pub fn from_file<P: AsRef<Path>>(file_path: P) -> Result<Self, ResolverError> {
        let file_path = file_path.as_ref();
        let json_str =
            std::fs::read_to_string(file_path).map_err(|source| ResolverError::ReadFile {
                path: file_path.display().to_string(),
                source,
            })?;

        let base_path = file_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .to_path_buf();

        Self::from_json_with_base(&json_str, base_path)
    }

    /// Create a new resolver from a resolver document
    /// Uses current directory as the base path for relative file references
    pub fn new(document: ResolverDocument) -> Result<Self, ResolverError> {
        Self::new_with_base(
            document,
            std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        )
    }

    /// Create a new resolver from a resolver document with a specific base path
    pub fn new_with_base<P: AsRef<Path>>(
        document: ResolverDocument,
        base_path: P,
    ) -> Result<Self, ResolverError> {
        Self::validate_document(&document)?;
        Ok(Resolver {
            document,
            base_path: base_path.as_ref().to_path_buf(),
        })
    }

    /// Validate the resolver document structure
    fn validate_document(document: &ResolverDocument) -> Result<(), ResolverError> {
        // Verify version
        if document.version != RESOLVER_VERSION {
            return Err(ResolverError::InvalidVersion {
                expected: RESOLVER_VERSION,
                found: document.version.clone(),
            });
        }

        // Verify resolution order exists
        if document.resolution_order.is_none() {
            return Err(ResolverError::MissingResolutionOrder);
        }

        if let Some(sets) = &document.sets {
            for (set_name, set) in sets {
                if set.sources.is_empty() {
                    return Err(ResolverError::InvalidReference {
                        message: format!("Set '{}' must contain at least one source", set_name),
                    });
                }
                Self::validate_sources(document, &set.sources, &format!("set '{}'", set_name))?;
            }
        }

        if let Some(modifiers) = &document.modifiers {
            for (modifier_name, modifier) in modifiers {
                if modifier.contexts.is_empty() {
                    return Err(ResolverError::InvalidReference {
                        message: format!(
                            "Modifier '{}' must declare at least one context",
                            modifier_name
                        ),
                    });
                }

                // The resolver spec says tools SHOULD throw when only one context is defined.
                if modifier.contexts.len() == 1 {
                    return Err(ResolverError::InvalidReference {
                        message: format!(
                            "Modifier '{}' should declare two or more contexts",
                            modifier_name
                        ),
                    });
                }

                if let Some(default) = &modifier.default
                    && !modifier.contexts.contains_key(default)
                {
                    return Err(ResolverError::InvalidReference {
                        message: format!(
                            "Modifier '{}' default '{}' is not present in contexts",
                            modifier_name, default
                        ),
                    });
                }

                for (context_name, sources) in &modifier.contexts {
                    Self::validate_sources(
                        document,
                        sources,
                        &format!("modifier '{}' context '{}'", modifier_name, context_name),
                    )?;
                }
            }
        }

        if let Some(resolution_order) = &document.resolution_order {
            let mut inline_names = HashSet::new();

            for item in resolution_order {
                use crate::ir::ResolutionItem;
                match item {
                    ResolutionItem::Reference { ref_path } => {
                        Self::validate_resolution_order_reference(document, ref_path)?;
                    }
                    ResolutionItem::InlineSet {
                        item_type,
                        name,
                        sources,
                        ..
                    } => {
                        if item_type != "set" {
                            return Err(ResolverError::InvalidReference {
                                message: format!(
                                    "Inline resolutionOrder item '{}' must have type 'set'",
                                    name
                                ),
                            });
                        }
                        if !inline_names.insert(name.clone()) {
                            return Err(ResolverError::InvalidReference {
                                message: format!(
                                    "Duplicate inline resolutionOrder name '{}'",
                                    name
                                ),
                            });
                        }
                        if sources.is_empty() {
                            return Err(ResolverError::InvalidReference {
                                message: format!(
                                    "Inline set '{}' must contain at least one source",
                                    name
                                ),
                            });
                        }
                        Self::validate_sources(
                            document,
                            sources,
                            &format!("inline set '{}'", name),
                        )?;
                    }
                    ResolutionItem::InlineModifier {
                        item_type,
                        name,
                        contexts,
                        default,
                        ..
                    } => {
                        if item_type != "modifier" {
                            return Err(ResolverError::InvalidReference {
                                message: format!(
                                    "Inline resolutionOrder item '{}' must have type 'modifier'",
                                    name
                                ),
                            });
                        }
                        if !inline_names.insert(name.clone()) {
                            return Err(ResolverError::InvalidReference {
                                message: format!(
                                    "Duplicate inline resolutionOrder name '{}'",
                                    name
                                ),
                            });
                        }
                        if contexts.is_empty() {
                            return Err(ResolverError::InvalidReference {
                                message: format!(
                                    "Inline modifier '{}' must declare at least one context",
                                    name
                                ),
                            });
                        }

                        // The resolver spec says tools SHOULD throw when only one context is defined.
                        if contexts.len() == 1 {
                            return Err(ResolverError::InvalidReference {
                                message: format!(
                                    "Inline modifier '{}' should declare two or more contexts",
                                    name
                                ),
                            });
                        }

                        if let Some(default_context) = default
                            && !contexts.contains_key(default_context)
                        {
                            return Err(ResolverError::InvalidReference {
                                message: format!(
                                    "Inline modifier '{}' default '{}' is not present in contexts",
                                    name, default_context
                                ),
                            });
                        }

                        for (context_name, sources) in contexts {
                            Self::validate_sources(
                                document,
                                sources,
                                &format!("inline modifier '{}' context '{}'", name, context_name),
                            )?;
                        }
                    }
                }
            }
        }

        Self::validate_no_set_reference_cycles(document)?;

        Ok(())
    }

    fn validate_sources(
        document: &ResolverDocument,
        sources: &[crate::ir::TokenSource],
        owner: &str,
    ) -> Result<(), ResolverError> {
        for source in sources {
            if let crate::ir::TokenSource::Reference { ref_path, .. } = source {
                Self::validate_source_reference(document, ref_path, owner)?;
            }
        }
        Ok(())
    }

    fn validate_source_reference(
        document: &ResolverDocument,
        ref_path: &str,
        owner: &str,
    ) -> Result<(), ResolverError> {
        let pointer_ref = JsonPointerRef::parse(ref_path);

        if pointer_ref.is_resolution_order_reference() {
            return Err(ResolverError::InvalidReference {
                message: format!(
                    "{} contains invalid reference '{}': references to resolutionOrder are not allowed",
                    owner, ref_path
                ),
            });
        }

        if !pointer_ref.is_same_document() {
            return Ok(());
        }

        if pointer_ref.is_modifier_reference() {
            return Err(ResolverError::InvalidReference {
                message: format!(
                    "{} contains invalid reference '{}': sets/modifier contexts cannot reference modifiers",
                    owner, ref_path
                ),
            });
        }

        if pointer_ref.is_set_reference() {
            if let Some(set_name) = pointer_ref.extract_name() {
                let exists = document
                    .sets
                    .as_ref()
                    .map(|sets| sets.contains_key(&set_name))
                    .unwrap_or(false);

                if !exists {
                    return Err(ResolverError::InvalidReference {
                        message: format!(
                            "{} references unknown set '{}' via '{}'",
                            owner, set_name, ref_path
                        ),
                    });
                }

                return Ok(());
            }
        }

        Err(ResolverError::InvalidReference {
            message: format!(
                "{} contains unsupported same-document reference '{}': expected '#/sets/<name>'",
                owner, ref_path
            ),
        })
    }

    fn validate_resolution_order_reference(
        document: &ResolverDocument,
        ref_path: &str,
    ) -> Result<(), ResolverError> {
        let pointer_ref = JsonPointerRef::parse(ref_path);

        if pointer_ref.is_resolution_order_reference() {
            return Err(ResolverError::InvalidReference {
                message: format!(
                    "Invalid reference '{}': references to resolutionOrder are not allowed",
                    ref_path
                ),
            });
        }

        if !pointer_ref.is_same_document() {
            return Ok(());
        }

        let Some(name) = pointer_ref.extract_name() else {
            return Err(ResolverError::InvalidReference {
                message: format!(
                    "Unsupported same-document reference '{}' in resolutionOrder",
                    ref_path
                ),
            });
        };

        if pointer_ref.is_set_reference() {
            let exists = document
                .sets
                .as_ref()
                .map(|sets| sets.contains_key(&name))
                .unwrap_or(false);

            if !exists {
                return Err(ResolverError::InvalidReference {
                    message: format!("resolutionOrder references unknown set '{}'", name),
                });
            }
            return Ok(());
        }

        if pointer_ref.is_modifier_reference() {
            let exists = document
                .modifiers
                .as_ref()
                .map(|modifiers| modifiers.contains_key(&name))
                .unwrap_or(false);

            if !exists {
                return Err(ResolverError::InvalidReference {
                    message: format!("resolutionOrder references unknown modifier '{}'", name),
                });
            }
            return Ok(());
        }

        Err(ResolverError::InvalidReference {
            message: format!(
                "Unsupported same-document reference '{}' in resolutionOrder",
                ref_path
            ),
        })
    }

    fn validate_no_set_reference_cycles(document: &ResolverDocument) -> Result<(), ResolverError> {
        let Some(sets) = &document.sets else {
            return Ok(());
        };

        let mut visiting = HashSet::new();
        let mut visited = HashSet::new();
        let mut stack = Vec::new();

        for set_name in sets.keys() {
            Self::visit_set_for_cycle_detection(
                document,
                set_name,
                &mut visiting,
                &mut visited,
                &mut stack,
            )?;
        }

        Ok(())
    }

    fn visit_set_for_cycle_detection(
        document: &ResolverDocument,
        set_name: &str,
        visiting: &mut HashSet<String>,
        visited: &mut HashSet<String>,
        stack: &mut Vec<String>,
    ) -> Result<(), ResolverError> {
        if visited.contains(set_name) {
            return Ok(());
        }

        if visiting.contains(set_name) {
            let start = stack.iter().position(|s| s == set_name).unwrap_or_default();
            let mut cycle = stack[start..].to_vec();
            cycle.push(set_name.to_string());
            return Err(ResolverError::CircularReference {
                message: cycle.join(" -> "),
            });
        }

        visiting.insert(set_name.to_string());
        stack.push(set_name.to_string());

        if let Some(set) = document.sets.as_ref().and_then(|sets| sets.get(set_name)) {
            for source in &set.sources {
                if let crate::ir::TokenSource::Reference { ref_path, .. } = source {
                    let pointer_ref = JsonPointerRef::parse(ref_path);
                    if pointer_ref.is_set_reference()
                        && pointer_ref.is_same_document()
                        && let Some(next_set_name) = pointer_ref.extract_name()
                    {
                        Self::visit_set_for_cycle_detection(
                            document,
                            &next_set_name,
                            visiting,
                            visited,
                            stack,
                        )?;
                    }
                }
            }
        }

        stack.pop();
        visiting.remove(set_name);
        visited.insert(set_name.to_string());
        Ok(())
    }

    /// Get the resolver document
    ///
    /// # Returns
    ///
    /// A reference to the resolver document used by this resolver instance
    pub fn document(&self) -> &ResolverDocument {
        &self.document
    }

    /// Get the base path used for resolving relative file references
    ///
    /// # Returns
    ///
    /// A reference to the base path as a `Path`
    pub fn base_path(&self) -> &Path {
        &self.base_path
    }

    /// Resolve tokens for given inputs
    /// Returns the merged token set after applying all transformations
    ///
    /// # Arguments
    ///
    /// * `input` - The resolution input specifying selected contexts and modifiers
    ///
    /// # Returns
    ///
    /// A `Value` containing the resolved tokens, or a `ResolverError` if resolution fails due to invalid input, references, or other issues.
    pub fn resolve(&self, input: ResolutionInput) -> Result<Value, ResolverError> {
        self.resolve_with_options(input, ResolveOptions::default())
    }

    /// Resolve tokens with explicit behavior toggles.
    ///
    /// Set `materialize_aliases` to false to preserve DTCG alias strings in
    /// output (useful for emitters that generate references such as CSS var()).
    ///
    /// Set `convert_token_refs_to_aliases` to true to rewrite token-level JSON
    /// refs (`$value: {"$ref": ...}`) into DTCG alias strings when the target
    /// points to another token node (or token `$value`).
    ///
    /// Set `materialize_property_refs` to false to preserve property-level JSON refs in output.
    /// Only really useful to set to false, if the output is able to handle JSON refs or has its
    /// own way to handle them. Most emitters will want to set this to true to get concrete values in the output.
    ///
    /// # Arguments
    ///
    /// * `input` - The resolution input specifying selected contexts and modifiers
    /// * `options` - Flags to control resolution behavior such as alias materialization and ref rewriting
    ///
    /// # Returns
    ///
    /// A `Value` containing the resolved tokens, or a `ResolverError` if resolution fails due to invalid input, references, or other issues.
    pub fn resolve_with_options(
        &self,
        input: ResolutionInput,
        options: ResolveOptions,
    ) -> Result<Value, ResolverError> {
        // Step 1: Validate input
        let validation_errors = input.validate(&self.document);
        if !validation_errors.is_empty() {
            return Err(ResolverError::InvalidInput {
                errors: validation_errors,
            });
        }

        // Step 2: Process resolution order
        let mut final_tokens = json!({});

        if let Some(resolution_order) = &self.document.resolution_order {
            for item in resolution_order {
                let item_tokens = self.process_resolution_item(item, &input)?;
                final_tokens = self.merge_tokens(final_tokens, item_tokens);
            }
        }

        // Step 3: Optionally resolve DTCG aliases after merge.
        if options.materialize_aliases {
            self.materialize_aliases(&mut final_tokens)?;
        }

        // Step 4: Materialize property-level JSON refs so emitters get concrete values.
        // Token-level refs (`$value: {"$ref": ...}`) are preserved for now.
        if options.materialize_property_refs {
            self.materialize_property_refs(&mut final_tokens)?;
        }

        // Step 5: Optionally rewrite token-level JSON refs to DTCG alias strings.
        if options.convert_token_refs_to_aliases {
            self.convert_token_refs_to_aliases(&mut final_tokens);
        }

        Ok(final_tokens)
    }

    /// Resolve, parse into IR, build graph, and validate graph in one call.
    pub fn resolve_to_graph_with_options(
        &self,
        input: ResolutionInput,
        options: ResolveOptions,
    ) -> Result<ResolvedGraphOutput, ResolverError> {
        let resolved_tokens = self.resolve_with_options(input, options)?;

        let mut parser_context = ParserContext::new(resolved_tokens.to_string());
        let document =
            parse_document(&mut parser_context).ok_or_else(|| ResolverError::InvalidReference {
                message: format!(
                    "Failed to parse resolved token JSON into IR ({} parser error(s))",
                    parser_context.errors.len()
                ),
            })?;

        let graph = TokenGraph::from_ir_document(&document);
        let graph_diagnostics = graph.validate(&document);

        Ok(ResolvedGraphOutput {
            resolved_tokens,
            document,
            graph,
            parser_diagnostics: parser_context.errors,
            graph_diagnostics,
        })
    }

    /// Resolve, parse into IR, build graph, and validate graph in one call
    /// using default resolve options.
    pub fn resolve_to_graph(
        &self,
        input: ResolutionInput,
    ) -> Result<ResolvedGraphOutput, ResolverError> {
        self.resolve_to_graph_with_options(input, ResolveOptions::default())
    }

    fn convert_token_refs_to_aliases(&self, tokens: &mut Value) {
        let snapshot = tokens.clone();
        self.convert_token_refs_to_aliases_in_document(tokens, &snapshot);
    }

    fn convert_token_refs_to_aliases_in_document(&self, node: &mut Value, snapshot: &Value) {
        match node {
            Value::Object(map) => {
                if let Some(token_value) = map.get_mut("$value")
                    && let Some(ref_path) = Self::extract_json_ref_path(token_value)
                    && let Some(target_segments) =
                        Self::alias_target_segments_from_ref(snapshot, ref_path)
                {
                    *token_value = Value::String(format!("{{{}}}", target_segments.join(".")));
                }

                let child_keys: Vec<String> = map
                    .keys()
                    .filter(|key| !key.starts_with('$'))
                    .cloned()
                    .collect();

                for key in child_keys {
                    if let Some(child) = map.get_mut(&key) {
                        self.convert_token_refs_to_aliases_in_document(child, snapshot);
                    }
                }
            }
            Value::Array(items) => {
                for item in items {
                    self.convert_token_refs_to_aliases_in_document(item, snapshot);
                }
            }
            _ => {}
        }
    }

    fn materialize_property_refs(&self, tokens: &mut Value) -> Result<(), ResolverError> {
        let snapshot = tokens.clone();
        let mut token_path = Vec::new();
        self.materialize_property_refs_in_document(tokens, &snapshot, &mut token_path)
    }

    fn materialize_property_refs_in_document(
        &self,
        node: &mut Value,
        snapshot: &Value,
        token_path: &mut Vec<String>,
    ) -> Result<(), ResolverError> {
        match node {
            Value::Object(map) => {
                if let Some(token_value) = map.get_mut("$value") {
                    let mut ref_stack = Vec::new();
                    self.materialize_property_refs_in_value(
                        token_value,
                        snapshot,
                        &mut ref_stack,
                        Some(token_path.as_slice()),
                        true,
                    )?;
                }

                let child_keys: Vec<String> = map
                    .keys()
                    .filter(|key| !key.starts_with('$'))
                    .cloned()
                    .collect();

                for key in child_keys {
                    if let Some(child) = map.get_mut(&key) {
                        token_path.push(key.clone());
                        self.materialize_property_refs_in_document(child, snapshot, token_path)?;
                        token_path.pop();
                    }
                }

                Ok(())
            }
            Value::Array(items) => {
                for item in items {
                    self.materialize_property_refs_in_document(item, snapshot, token_path)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn materialize_property_refs_in_value(
        &self,
        value: &mut Value,
        snapshot: &Value,
        ref_stack: &mut Vec<String>,
        source_token_path: Option<&[String]>,
        is_token_value_root: bool,
    ) -> Result<(), ResolverError> {
        if !is_token_value_root && let Some(ref_path) = Self::extract_json_ref_path(value) {
            let mut resolved = self.resolve_json_ref_target_value(
                snapshot,
                ref_path,
                ref_stack,
                source_token_path,
            )?;

            self.materialize_property_refs_in_value(
                &mut resolved,
                snapshot,
                ref_stack,
                source_token_path,
                false,
            )?;

            *value = resolved;
            return Ok(());
        }

        match value {
            Value::Object(map) => {
                let keys: Vec<String> = map.keys().cloned().collect();
                for key in keys {
                    if let Some(child) = map.get_mut(&key) {
                        self.materialize_property_refs_in_value(
                            child,
                            snapshot,
                            ref_stack,
                            source_token_path,
                            false,
                        )?;
                    }
                }
                Ok(())
            }
            Value::Array(items) => {
                for item in items {
                    self.materialize_property_refs_in_value(
                        item,
                        snapshot,
                        ref_stack,
                        source_token_path,
                        false,
                    )?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn extract_json_ref_path(value: &Value) -> Option<&str> {
        let Value::Object(map) = value else {
            return None;
        };

        if map.len() != 1 {
            return None;
        }

        map.get("$ref").and_then(Value::as_str)
    }

    fn alias_target_segments_from_ref(snapshot: &Value, ref_path: &str) -> Option<Vec<String>> {
        let pointer_ref = JsonPointerRef::parse(ref_path);
        if !pointer_ref.is_same_document() || pointer_ref.file_path.is_some() {
            return None;
        }

        let fragment = pointer_ref.fragment.as_deref().unwrap_or("");
        let segments = Self::decode_json_pointer_segments(fragment);
        if segments.is_empty() {
            return None;
        }

        if Self::is_token_node_at_path(snapshot, &segments) {
            return Some(segments);
        }

        if segments.last().map(|segment| segment == "$value") == Some(true) && segments.len() > 1 {
            let token_segments = segments[..segments.len() - 1].to_vec();
            if Self::is_token_node_at_path(snapshot, &token_segments) {
                return Some(token_segments);
            }
        }

        None
    }

    fn is_token_node_at_path(snapshot: &Value, segments: &[String]) -> bool {
        Self::lookup_node(snapshot, segments)
            .map(|node| node.contains_key("$value"))
            .unwrap_or(false)
    }

    fn decode_json_pointer_segments(fragment: &str) -> Vec<String> {
        fragment
            .trim_start_matches('/')
            .split('/')
            .filter(|segment| !segment.is_empty())
            .map(|segment| segment.replace("~1", "/").replace("~0", "~"))
            .collect()
    }

    fn resolve_json_ref_target_value(
        &self,
        snapshot: &Value,
        ref_path: &str,
        ref_stack: &mut Vec<String>,
        source_token_path: Option<&[String]>,
    ) -> Result<Value, ResolverError> {
        if ref_stack.iter().any(|visited| visited == ref_path) {
            let mut cycle = ref_stack.clone();
            cycle.push(ref_path.to_string());
            return Err(ResolverError::CircularReference {
                message: cycle.join(" -> "),
            });
        }

        let source_display = source_token_path
            .map(|segments| {
                if segments.is_empty() {
                    "<root>".to_string()
                } else {
                    segments.join(".")
                }
            })
            .unwrap_or_else(|| "<unknown>".to_string());

        let pointer_ref = JsonPointerRef::parse(ref_path);
        let resolved = if pointer_ref.is_same_document() {
            let fragment = pointer_ref.fragment.as_deref().unwrap_or("");
            self.navigate_json_pointer(snapshot, fragment)?
        } else {
            self.load_external_reference(&pointer_ref)?
        };

        ref_stack.push(ref_path.to_string());
        let mut resolved_value = resolved;
        self.materialize_property_refs_in_value(
            &mut resolved_value,
            snapshot,
            ref_stack,
            source_token_path,
            false,
        )?;
        ref_stack.pop();

        if resolved_value.is_null() {
            return Err(ResolverError::InvalidReference {
                message: format!(
                    "Unresolved property reference '{}' referenced from '{}'",
                    ref_path, source_display
                ),
            });
        }

        Ok(resolved_value)
    }

    fn materialize_aliases(&self, tokens: &mut Value) -> Result<(), ResolverError> {
        let snapshot = tokens.clone();
        let mut token_path = Vec::new();
        self.materialize_aliases_in_document(tokens, &snapshot, &mut token_path)
    }

    fn materialize_aliases_in_document(
        &self,
        node: &mut Value,
        snapshot: &Value,
        token_path: &mut Vec<String>,
    ) -> Result<(), ResolverError> {
        match node {
            Value::Object(map) => {
                if let Some(token_value) = map.get_mut("$value") {
                    let mut alias_stack = Vec::new();
                    self.materialize_aliases_in_value(
                        token_value,
                        snapshot,
                        &mut alias_stack,
                        Some(token_path.as_slice()),
                    )?;
                }

                let child_keys: Vec<String> = map
                    .keys()
                    .filter(|key| !key.starts_with('$'))
                    .cloned()
                    .collect();

                for key in child_keys {
                    if let Some(child) = map.get_mut(&key) {
                        token_path.push(key.clone());
                        self.materialize_aliases_in_document(child, snapshot, token_path)?;
                        token_path.pop();
                    }
                }

                Ok(())
            }
            Value::Array(items) => {
                for item in items {
                    self.materialize_aliases_in_document(item, snapshot, token_path)?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn materialize_aliases_in_value(
        &self,
        value: &mut Value,
        snapshot: &Value,
        alias_stack: &mut Vec<String>,
        source_token_path: Option<&[String]>,
    ) -> Result<(), ResolverError> {
        match value {
            Value::String(raw) => {
                if let Some(alias) = TokenAlias::from_dtcg_alias(raw) {
                    let mut resolved = self.resolve_alias_target_value(
                        snapshot,
                        &alias.target_path.segments,
                        alias_stack,
                        source_token_path,
                    )?;

                    self.materialize_aliases_in_value(
                        &mut resolved,
                        snapshot,
                        alias_stack,
                        source_token_path,
                    )?;

                    *value = resolved;
                }

                Ok(())
            }
            Value::Object(map) => {
                let keys: Vec<String> = map.keys().cloned().collect();
                for key in keys {
                    if let Some(child) = map.get_mut(&key) {
                        self.materialize_aliases_in_value(
                            child,
                            snapshot,
                            alias_stack,
                            source_token_path,
                        )?;
                    }
                }
                Ok(())
            }
            Value::Array(items) => {
                for item in items {
                    self.materialize_aliases_in_value(
                        item,
                        snapshot,
                        alias_stack,
                        source_token_path,
                    )?;
                }
                Ok(())
            }
            _ => Ok(()),
        }
    }

    fn resolve_alias_target_value(
        &self,
        snapshot: &Value,
        target_segments: &[String],
        alias_stack: &mut Vec<String>,
        source_token_path: Option<&[String]>,
    ) -> Result<Value, ResolverError> {
        let target_key = if target_segments.is_empty() {
            "<root>".to_string()
        } else {
            target_segments.join(".")
        };

        if alias_stack.contains(&target_key) {
            let mut cycle = alias_stack.clone();
            cycle.push(target_key);
            return Err(ResolverError::CircularReference {
                message: cycle.join(" -> "),
            });
        }

        let source_display = source_token_path
            .map(|segments| {
                if segments.is_empty() {
                    "<root>".to_string()
                } else {
                    segments.join(".")
                }
            })
            .unwrap_or_else(|| "<unknown>".to_string());

        let target_node = Self::lookup_node(snapshot, target_segments).ok_or_else(|| {
            ResolverError::InvalidReference {
                message: format!(
                    "Unresolved alias target '{{{}}}' referenced from '{}'",
                    target_segments.join("."),
                    source_display
                ),
            }
        })?;

        let mut target_value =
            target_node
                .get("$value")
                .cloned()
                .ok_or_else(|| ResolverError::InvalidReference {
                    message: format!(
                        "Alias target '{{{}}}' referenced from '{}' does not point to a token",
                        target_segments.join("."),
                        source_display
                    ),
                })?;

        alias_stack.push(if target_segments.is_empty() {
            "<root>".to_string()
        } else {
            target_segments.join(".")
        });
        self.materialize_aliases_in_value(
            &mut target_value,
            snapshot,
            alias_stack,
            Some(target_segments),
        )?;
        alias_stack.pop();

        Ok(target_value)
    }

    fn lookup_node<'a>(
        root: &'a Value,
        path: &[String],
    ) -> Option<&'a serde_json::Map<String, Value>> {
        let mut current = root;

        for segment in path {
            let map = current.as_object()?;
            current = map.get(segment)?;
        }

        current.as_object()
    }

    /// Process a single resolution item (set or modifier)
    ///
    /// # Arguments
    ///
    /// * `item` - The resolution item to process, which can be a reference, inline set, or inline modifier
    /// * `input` - The resolution input providing context for processing (e.g., selected modifier contexts)
    ///
    /// # Returns
    ///
    /// A `Value` containing the resolved tokens for the given resolution item, or a `ResolverError` if processing fails due to invalid
    /// references, contexts, or other issues.
    fn process_resolution_item(
        &self,
        item: &crate::ir::ResolutionItem,
        input: &ResolutionInput,
    ) -> Result<Value, ResolverError> {
        use crate::ir::ResolutionItem;

        match item {
            ResolutionItem::Reference { ref_path } => {
                let mut resolved = self.resolve_reference(ref_path, input)?;
                let pointer_ref = JsonPointerRef::parse(ref_path);
                if !pointer_ref.is_same_document() {
                    self.annotate_with_provenance(&mut resolved, ref_path, "#");
                }
                Ok(resolved)
            }
            ResolutionItem::InlineSet { sources, .. } => self.merge_sources(sources, input),
            ResolutionItem::InlineModifier {
                name,
                contexts,
                default,
                ..
            } => {
                // Get the context value from input or use default
                let context_key = input
                    .selections
                    .get(name)
                    .cloned()
                    .or_else(|| default.clone())
                    .ok_or_else(|| ResolverError::InvalidInput {
                        errors: vec![format!("Missing modifier '{}'", name)],
                    })?;

                if let Some(sources) = contexts.get(&context_key) {
                    self.merge_sources(sources, input)
                } else {
                    Err(ResolverError::InvalidInput {
                        errors: vec![format!(
                            "Invalid context '{}' for modifier '{}'",
                            context_key, name
                        )],
                    })
                }
            }
        }
    }

    /// Resolve a reference object
    ///
    /// # Arguments
    ///
    /// * `ref_path` - The JSON pointer reference path to resolve
    /// * `input` - The resolution input for context
    ///
    /// # Returns
    ///
    /// A `Value` containing the resolved reference content, or a `ResolverError` if
    /// the reference is invalid, points to an unsupported location, or if loading/parsing an external reference fails.
    fn resolve_reference(
        &self,
        ref_path: &str,
        input: &ResolutionInput,
    ) -> Result<Value, ResolverError> {
        let pointer_ref = JsonPointerRef::parse(ref_path);

        // Check for invalid references
        if pointer_ref.is_resolution_order_reference() {
            return Err(ResolverError::InvalidReference {
                message: "Cannot reference resolutionOrder items".to_string(),
            });
        }

        if pointer_ref.is_same_document() {
            // Same-document reference
            if let Some(name) = pointer_ref.extract_name() {
                if pointer_ref.is_set_reference() {
                    // Reference to a set
                    if let Some(sets) = &self.document.sets {
                        if let Some(set) = sets.get(&name) {
                            return self.merge_sources(&set.sources, input);
                        }
                    }
                    return Err(ResolverError::InvalidReference {
                        message: format!("Set '{}' not found", name),
                    });
                } else if pointer_ref.is_modifier_reference() {
                    // Reference to a modifier
                    if let Some(modifiers) = &self.document.modifiers {
                        if let Some(modifier) = modifiers.get(&name) {
                            // Determine which context to use
                            let context_key = input
                                .selections
                                .get(&name)
                                .cloned()
                                .or_else(|| modifier.default.clone())
                                .ok_or_else(|| ResolverError::InvalidInput {
                                    errors: vec![format!("Missing modifier '{}'", name)],
                                })?;

                            if let Some(sources) = modifier.contexts.get(&context_key) {
                                return self.merge_sources(sources, input);
                            } else {
                                return Err(ResolverError::InvalidReference {
                                    message: format!(
                                        "Invalid context '{}' for modifier '{}'",
                                        context_key, name
                                    ),
                                });
                            }
                        }
                    }
                    return Err(ResolverError::InvalidReference {
                        message: format!("Modifier '{}' not found", name),
                    });
                }
            }
        } else {
            // External file reference
            return self.load_external_reference(&pointer_ref);
        }

        Err(ResolverError::InvalidReference {
            message: format!("Could not resolve reference '{}'", ref_path),
        })
    }

    /// Load and parse an external file reference
    ///
    /// # Arguments
    ///
    /// * `pointer_ref` - The JSON pointer reference containing the file path and optional fragment
    ///
    /// # Returns
    ///
    /// A `Value` containing the loaded and parsed JSON content from the external file, or a `ResolverError` if loading or parsing fails.
    fn load_external_reference(
        &self,
        pointer_ref: &JsonPointerRef,
    ) -> Result<Value, ResolverError> {
        let file_path =
            pointer_ref
                .file_path
                .as_ref()
                .ok_or_else(|| ResolverError::InvalidReference {
                    message: "No file path in reference".to_string(),
                })?;

        // Resolve relative path based on base_path
        let full_path = if Path::new(file_path).is_absolute() {
            PathBuf::from(file_path)
        } else {
            self.base_path.join(file_path)
        };

        // Load the file
        let json_str =
            std::fs::read_to_string(&full_path).map_err(|source| ResolverError::ReadFile {
                path: full_path.display().to_string(),
                source,
            })?;

        // Parse JSON
        let mut file_content: Value =
            serde_json::from_str(&json_str).map_err(|source| ResolverError::ParseExternalJson {
                path: full_path.display().to_string(),
                source,
            })?;

        // Navigate to the fragment if present
        if let Some(fragment) = &pointer_ref.fragment {
            file_content = self.navigate_json_pointer(&file_content, fragment)?;
        }

        Ok(file_content)
    }

    /// Navigate a JSON object using a JSON Pointer (RFC 6901)
    fn navigate_json_pointer(&self, value: &Value, pointer: &str) -> Result<Value, ResolverError> {
        if pointer.is_empty() || pointer == "/" {
            return Ok(value.clone());
        }

        let parts: Vec<&str> = pointer.split('/').filter(|p| !p.is_empty()).collect();
        let mut current = value.clone();

        for part in parts {
            // Unescape JSON Pointer special characters
            let unescaped = part.replace("~1", "/").replace("~0", "~");

            match &current {
                Value::Object(map) => {
                    current = map.get(&unescaped).cloned().ok_or_else(|| {
                        ResolverError::InvalidJsonPointer {
                            pointer: pointer.to_string(),
                            message: format!("Property '{}' not found in JSON", unescaped),
                        }
                    })?;
                }
                Value::Array(arr) => {
                    let idx = unescaped.parse::<usize>().map_err(|_| {
                        ResolverError::InvalidJsonPointer {
                            pointer: pointer.to_string(),
                            message: format!("Invalid array index: {}", unescaped),
                        }
                    })?;
                    current =
                        arr.get(idx)
                            .cloned()
                            .ok_or_else(|| ResolverError::InvalidJsonPointer {
                                pointer: pointer.to_string(),
                                message: format!("Array index {} out of bounds", idx),
                            })?;
                }
                _ => {
                    return Err(ResolverError::InvalidJsonPointer {
                        pointer: pointer.to_string(),
                        message: "Cannot navigate into non-object/array value".to_string(),
                    });
                }
            }
        }

        Ok(current)
    }

    /// Merge multiple token sources in order (later overrides earlier)
    fn merge_sources(
        &self,
        sources: &[crate::ir::TokenSource],
        input: &ResolutionInput,
    ) -> Result<Value, ResolverError> {
        let mut result = json!({});

        for (source_index, source) in sources.iter().enumerate() {
            use crate::ir::TokenSource;
            match source {
                TokenSource::Reference {
                    ref_path,
                    overrides,
                } => {
                    let mut source_tokens = self.resolve_reference(ref_path, input)?;
                    self.annotate_with_provenance(&mut source_tokens, ref_path, "#");

                    // Apply overrides if present
                    if let Some(overrides_obj) = overrides {
                        source_tokens = self.merge_tokens(source_tokens, overrides_obj.clone());
                    }

                    result = self.merge_tokens(result, source_tokens);
                }
                TokenSource::Inline(inline_tokens) => {
                    let mut inline_with_provenance = inline_tokens.clone();
                    let inline_source = format!("$inline/{}", source_index);
                    self.annotate_with_provenance(&mut inline_with_provenance, &inline_source, "#");
                    result = self.merge_tokens(result, inline_with_provenance);
                }
            }
        }

        Ok(result)
    }

    fn annotate_with_provenance(&self, value: &mut Value, source: &str, pointer: &str) {
        match value {
            Value::Object(map) => {
                let is_token_node = map.contains_key("$value");
                let is_group_node = map.keys().any(|key| !key.starts_with('$'));

                if is_token_node || is_group_node {
                    let extensions_value = map
                        .entry("$extensions".to_string())
                        .or_insert_with(|| Value::Object(serde_json::Map::new()));

                    if !extensions_value.is_object() {
                        *extensions_value = Value::Object(serde_json::Map::new());
                    }

                    if let Value::Object(extensions) = extensions_value {
                        extensions.insert(
                            PROVENANCE_EXTENSION_KEY.to_string(),
                            json!({
                                "source": source,
                                "pointer": pointer,
                            }),
                        );
                    }
                }

                // Token value objects are literals, not nested token/group trees.
                if is_token_node {
                    return;
                }

                let keys: Vec<String> = map
                    .keys()
                    .filter(|key| !key.starts_with('$'))
                    .cloned()
                    .collect();
                for key in keys {
                    if let Some(child) = map.get_mut(&key) {
                        let escaped_key = key.replace('~', "~0").replace('/', "~1");
                        let child_pointer = if pointer == "#" {
                            format!("#/{}", escaped_key)
                        } else {
                            format!("{}/{}", pointer, escaped_key)
                        };
                        self.annotate_with_provenance(child, source, &child_pointer);
                    }
                }
            }
            Value::Array(items) => {
                for (index, item) in items.iter_mut().enumerate() {
                    let child_pointer = if pointer == "#" {
                        format!("#/{}", index)
                    } else {
                        format!("{}/{}", pointer, index)
                    };
                    self.annotate_with_provenance(item, source, &child_pointer);
                }
            }
            _ => {}
        }
    }

    /// Deep merge two JSON objects
    /// Later (right) values override earlier (left) values
    fn merge_tokens(&self, mut base: Value, update: Value) -> Value {
        match (&mut base, update) {
            (Value::Object(base_map), Value::Object(update_map)) => {
                for (key, value) in update_map {
                    if let Some(base_value) = base_map.remove(&key) {
                        base_map.insert(key, self.merge_tokens(base_value, value));
                    } else {
                        base_map.insert(key, value);
                    }
                }
                base
            }
            (_, update) => update, // Later override completely
        }
    }

    /// Get all possible resolution permutations
    /// Useful for generating all variants of tokens
    pub fn get_all_permutations(&self) -> Vec<ResolutionInput> {
        let mut permutations = vec![ResolutionInput::new()];

        if let Some(modifiers) = &self.document.modifiers {
            for (modifier_name, modifier) in modifiers {
                let mut new_permutations = Vec::new();

                for context_name in modifier.contexts.keys() {
                    for mut perm in permutations.clone() {
                        perm.add_selection(modifier_name.clone(), context_name.clone());
                        new_permutations.push(perm);
                    }
                }

                permutations = new_permutations;
            }
        }

        permutations
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn srgb_color(hex: &str, components: [f64; 3]) -> serde_json::Value {
        serde_json::json!({
            "$value": {
                "colorSpace": "srgb",
                "components": components,
                "hex": hex
            },
            "$type": "color"
        })
    }

    fn srgb_color_value(hex: &str, components: [f64; 3]) -> serde_json::Value {
        serde_json::json!({
            "colorSpace": "srgb",
            "components": components,
            "hex": hex
        })
    }

    #[test]
    fn test_resolver_creation_invalid_version() {
        let json = r#"{"version": "2024.10", "resolutionOrder": []}"#;
        let result = Resolver::from_json(json);
        assert!(matches!(result, Err(ResolverError::InvalidVersion { .. })));
    }

    #[test]
    fn test_resolver_creation_missing_resolution_order() {
        let json = r#"{"version": "2025.10"}"#;
        let result = Resolver::from_json(json);
        assert!(matches!(result, Err(ResolverError::MissingResolutionOrder)));
    }

    #[test]
    fn test_simple_resolver_with_sets() {
        // Build JSON using serde_json to avoid raw string parsing issues
        let json = serde_json::json!({
            "version": "2025.10",
            "sets": {
                "base": {
                    "sources": [
                        { "color": { "primary": srgb_color("#FF0000", [1.0, 0.0, 0.0]) } }
                    ]
                }
            },
            "resolutionOrder": [
                { "$ref": "#/sets/base" }
            ]
        });

        let resolver = Resolver::from_json(&json.to_string()).expect("Failed to create resolver");
        let input = ResolutionInput::new();
        let result = resolver.resolve(input).expect("Failed to resolve");

        assert!(result.get("color").is_some());
        assert!(result.get("color").unwrap().get("primary").is_some());
    }

    #[test]
    fn test_resolver_with_modifiers() {
        let json = serde_json::json!({
            "version": "2025.10",
            "sets": {
                "foundation": {
                    "sources": [
                        {
                            "space": {
                                "small": {
                                    "$value": { "value": 4, "unit": "px" },
                                    "$type": "dimension"
                                }
                            }
                        }
                    ]
                }
            },
            "modifiers": {
                "theme": {
                    "contexts": {
                        "light": [
                            {
                                "color": {
                                    "bg": srgb_color("#FFFFFF", [1.0, 1.0, 1.0])
                                }
                            }
                        ],
                        "dark": [
                            {
                                "color": {
                                    "bg": srgb_color("#000000", [0.0, 0.0, 0.0])
                                }
                            }
                        ]
                    },
                    "default": "light"
                }
            },
            "resolutionOrder": [
                { "$ref": "#/sets/foundation" },
                { "$ref": "#/modifiers/theme" }
            ]
        });

        let resolver = Resolver::from_json(&json.to_string()).expect("Failed to create resolver");

        // Test with light theme
        let mut light_input = ResolutionInput::new();
        light_input.add_selection("theme".to_string(), "light".to_string());
        let light_result = resolver
            .resolve(light_input)
            .expect("Failed to resolve light theme");

        assert_eq!(
            light_result
                .get("color")
                .unwrap()
                .get("bg")
                .unwrap()
                .get("$value")
                .unwrap(),
            &srgb_color_value("#FFFFFF", [1.0, 1.0, 1.0])
        );

        // Test with dark theme
        let mut dark_input = ResolutionInput::new();
        dark_input.add_selection("theme".to_string(), "dark".to_string());
        let dark_result = resolver
            .resolve(dark_input)
            .expect("Failed to resolve dark theme");

        assert_eq!(
            dark_result
                .get("color")
                .unwrap()
                .get("bg")
                .unwrap()
                .get("$value")
                .unwrap(),
            &srgb_color_value("#000000", [0.0, 0.0, 0.0])
        );
    }

    #[test]
    fn test_resolver_default_modifier() {
        let json = serde_json::json!({
            "version": "2025.10",
            "modifiers": {
                "theme": {
                    "contexts": {
                        "light": [{ "color": { "bg": srgb_color("#FFF", [1.0, 1.0, 1.0]) } }],
                        "dark": [{ "color": { "bg": srgb_color("#000", [0.0, 0.0, 0.0]) } }]
                    },
                    "default": "light"
                }
            },
            "resolutionOrder": [
                { "$ref": "#/modifiers/theme" }
            ]
        });

        let resolver = Resolver::from_json(&json.to_string()).expect("Failed to create resolver");

        // Use default without providing input
        let empty_input = ResolutionInput::new();
        let result = resolver
            .resolve(empty_input)
            .expect("Failed to resolve with default");

        assert_eq!(
            result
                .get("color")
                .unwrap()
                .get("bg")
                .unwrap()
                .get("$value")
                .unwrap(),
            &srgb_color_value("#FFF", [1.0, 1.0, 1.0])
        );
    }

    #[test]
    fn test_resolver_multiple_modifiers() {
        let json = serde_json::json!({
            "version": "2025.10",
            "modifiers": {
                "theme": {
                    "contexts": {
                        "light": [{ "color": { "bg": srgb_color("#FFF", [1.0, 1.0, 1.0]) } }],
                        "dark": [{ "color": { "bg": srgb_color("#000", [0.0, 0.0, 0.0]) } }]
                    }
                },
                "size": {
                    "contexts": {
                        "small": [{ "space": { "base": { "$value": { "value": 4, "unit": "px" } } } }],
                        "large": [{ "space": { "base": { "$value": { "value": 8, "unit": "px" } } } }]
                    }
                }
            },
            "resolutionOrder": [
                { "$ref": "#/modifiers/theme" },
                { "$ref": "#/modifiers/size" }
            ]
        });

        let resolver = Resolver::from_json(&json.to_string()).expect("Failed to create resolver");

        let mut input = ResolutionInput::new();
        input.add_selection("theme".to_string(), "light".to_string());
        input.add_selection("size".to_string(), "large".to_string());
        let result = resolver.resolve(input).expect("Failed to resolve");

        assert_eq!(
            result
                .get("color")
                .unwrap()
                .get("bg")
                .unwrap()
                .get("$value")
                .unwrap(),
            &srgb_color_value("#FFF", [1.0, 1.0, 1.0])
        );
        assert_eq!(
            result
                .get("space")
                .unwrap()
                .get("base")
                .unwrap()
                .get("$value")
                .unwrap(),
            &serde_json::json!({ "value": 8, "unit": "px" })
        );
    }

    #[test]
    fn test_resolver_input_validation() {
        let json = serde_json::json!({
            "version": "2025.10",
            "modifiers": {
                "theme": {
                    "contexts": {
                        "light": [{ "color": { "bg": srgb_color("#FFF", [1.0, 1.0, 1.0]) } }],
                        "dark": [{ "color": { "bg": srgb_color("#000", [0.0, 0.0, 0.0]) } }]
                    }
                }
            },
            "resolutionOrder": [
                { "$ref": "#/modifiers/theme" }
            ]
        });

        let resolver = Resolver::from_json(&json.to_string()).expect("Failed to create resolver");

        // Test invalid context
        let mut invalid_input = ResolutionInput::new();
        invalid_input.add_selection("theme".to_string(), "invalid".to_string());
        let result = resolver.resolve(invalid_input);
        assert!(result.is_err());

        // Test unknown modifier
        let mut unknown_input = ResolutionInput::new();
        unknown_input.add_selection("unknown".to_string(), "value".to_string());
        let result = resolver.resolve(unknown_input);
        assert!(result.is_err());
    }

    #[test]
    fn test_json_pointer_navigation() {
        let json = serde_json::json!({
            "version": "2025.10",
            "sets": {
                "base": {
                    "sources": [{ "colors": { "primary": srgb_color("#FF0000", [1.0, 0.0, 0.0]) } }]
                }
            },
            "resolutionOrder": [{ "$ref": "#/sets/base" }]
        });

        let resolver = Resolver::from_json(&json.to_string()).expect("Failed to create resolver");
        let input = ResolutionInput::new();
        let result = resolver.resolve(input).expect("Failed to resolve");

        assert_eq!(
            result
                .get("colors")
                .unwrap()
                .get("primary")
                .unwrap()
                .get("$value")
                .unwrap(),
            &srgb_color_value("#FF0000", [1.0, 0.0, 0.0])
        );
    }

    #[test]
    fn test_external_file_reference() {
        // Create a temporary tokens file
        let temp_dir = std::env::temp_dir();
        let tokens_file = temp_dir.join("test_tokens.json");
        let external_tokens = serde_json::json!({
            "color": {
                "primary": srgb_color("#0066CC", [0.0, 0.4, 0.8]),
                "secondary": srgb_color("#6C757D", [0.4235, 0.4588, 0.4902])
            }
        });
        std::fs::write(&tokens_file, external_tokens.to_string())
            .expect("Failed to write test file");

        // Create resolver that references the external file
        let json = serde_json::json!({
            "version": "2025.10",
            "resolutionOrder": [
                { "$ref": "test_tokens.json" }
            ]
        });

        let resolver = Resolver::from_json_with_base(&json.to_string(), &temp_dir)
            .expect("Failed to create resolver");
        let input = ResolutionInput::new();
        let result = resolver.resolve(input).expect("Failed to resolve");

        assert_eq!(
            result
                .get("color")
                .unwrap()
                .get("primary")
                .unwrap()
                .get("$value")
                .unwrap(),
            &srgb_color_value("#0066CC", [0.0, 0.4, 0.8])
        );
        assert_eq!(
            result
                .get("color")
                .unwrap()
                .get("secondary")
                .unwrap()
                .get("$value")
                .unwrap(),
            &srgb_color_value("#6C757D", [0.4235, 0.4588, 0.4902])
        );

        // Cleanup
        let _ = std::fs::remove_file(&tokens_file);
    }

    #[test]
    fn test_external_file_with_fragment() {
        // Create a temporary tokens file
        let temp_dir = std::env::temp_dir();
        let tokens_file = temp_dir.join("test_tokens_fragment.json");
        let external_tokens = serde_json::json!({
            "foundation": {
                "colors": {
                    "primary": srgb_color("#FF6B6B", [1.0, 0.4196, 0.4196])
                }
            },
            "semantic": {
                "button": srgb_color("#4ECDC4", [0.3059, 0.8039, 0.7686])
            }
        });
        std::fs::write(&tokens_file, external_tokens.to_string())
            .expect("Failed to write test file");

        // Create resolver that references a fragment of the external file
        let json = serde_json::json!({
            "version": "2025.10",
            "resolutionOrder": [
                { "$ref": "test_tokens_fragment.json#/foundation/colors" }
            ]
        });

        let resolver = Resolver::from_json_with_base(&json.to_string(), &temp_dir)
            .expect("Failed to create resolver");
        let input = ResolutionInput::new();
        let result = resolver.resolve(input).expect("Failed to resolve");

        // Only the fragment should be included
        assert_eq!(
            result.get("primary").unwrap().get("$value").unwrap(),
            &srgb_color_value("#FF6B6B", [1.0, 0.4196, 0.4196])
        );

        let provenance = result
            .get("primary")
            .and_then(|node| node.get("$extensions"))
            .and_then(|extensions| extensions.get(PROVENANCE_EXTENSION_KEY))
            .expect("resolved node should include provenance metadata");

        assert_eq!(
            provenance.get("source").and_then(serde_json::Value::as_str),
            Some("test_tokens_fragment.json#/foundation/colors")
        );
        assert_eq!(
            provenance
                .get("pointer")
                .and_then(serde_json::Value::as_str),
            Some("#/primary")
        );

        // The semantic button should not be present
        assert!(result.get("button").is_none());

        // Cleanup
        let _ = std::fs::remove_file(&tokens_file);
    }

    #[test]
    fn test_inline_sources_include_provenance_and_late_override_wins() {
        let json = serde_json::json!({
            "version": "2025.10",
            "sets": {
                "base": {
                    "sources": [
                        {
                            "color": {
                                "primary": srgb_color("#FF0000", [1.0, 0.0, 0.0])
                            }
                        },
                        {
                            "color": {
                                "primary": srgb_color("#00FF00", [0.0, 1.0, 0.0])
                            }
                        }
                    ]
                }
            },
            "resolutionOrder": [
                { "$ref": "#/sets/base" }
            ]
        });

        let resolver = Resolver::from_json(&json.to_string()).expect("Failed to create resolver");
        let result = resolver
            .resolve(ResolutionInput::new())
            .expect("Failed to resolve");

        let provenance = result
            .get("color")
            .and_then(|group| group.get("primary"))
            .and_then(|node| node.get("$extensions"))
            .and_then(|extensions| extensions.get(PROVENANCE_EXTENSION_KEY))
            .expect("resolved inline token should include provenance metadata");

        assert_eq!(
            provenance.get("source").and_then(serde_json::Value::as_str),
            Some("$inline/1")
        );
        assert_eq!(
            provenance
                .get("pointer")
                .and_then(serde_json::Value::as_str),
            Some("#/color/primary")
        );
    }

    #[test]
    fn test_external_file_not_found() {
        let json = serde_json::json!({
            "version": "2025.10",
            "resolutionOrder": [
                { "$ref": "nonexistent_file.json" }
            ]
        });

        let resolver = Resolver::from_json(&json.to_string()).expect("Failed to create resolver");
        let input = ResolutionInput::new();
        let result = resolver.resolve(input);

        assert!(matches!(result, Err(ResolverError::ReadFile { .. })));
    }

    #[test]
    fn test_modifier_default_must_exist_in_contexts() {
        let json = serde_json::json!({
            "version": "2025.10",
            "modifiers": {
                "theme": {
                    "contexts": {
                        "light": [{ "color": { "bg": srgb_color("#fff", [1.0, 1.0, 1.0]) } }],
                        "dark": [{ "color": { "bg": srgb_color("#000", [0.0, 0.0, 0.0]) } }]
                    },
                    "default": "missing"
                }
            },
            "resolutionOrder": [
                { "$ref": "#/modifiers/theme" }
            ]
        });

        let result = Resolver::from_json(&json.to_string());
        assert!(matches!(
            result,
            Err(ResolverError::InvalidReference { .. })
        ));
    }

    #[test]
    fn test_modifier_must_have_two_or_more_contexts() {
        let json = serde_json::json!({
            "version": "2025.10",
            "modifiers": {
                "theme": {
                    "contexts": {
                        "light": [{ "color": { "bg": srgb_color("#fff", [1.0, 1.0, 1.0]) } }]
                    }
                }
            },
            "resolutionOrder": [
                { "$ref": "#/modifiers/theme" }
            ]
        });

        let result = Resolver::from_json(&json.to_string());
        assert!(matches!(
            result,
            Err(ResolverError::InvalidReference { .. })
        ));
    }

    #[test]
    fn test_set_source_must_not_reference_modifier() {
        let json = serde_json::json!({
            "version": "2025.10",
            "sets": {
                "base": {
                    "sources": [{ "$ref": "#/modifiers/theme" }]
                }
            },
            "modifiers": {
                "theme": {
                    "contexts": {
                        "light": [{ "color": { "bg": srgb_color("#fff", [1.0, 1.0, 1.0]) } }],
                        "dark": [{ "color": { "bg": srgb_color("#000", [0.0, 0.0, 0.0]) } }]
                    }
                }
            },
            "resolutionOrder": [
                { "$ref": "#/sets/base" }
            ]
        });

        let result = Resolver::from_json(&json.to_string());
        assert!(matches!(
            result,
            Err(ResolverError::InvalidReference { .. })
        ));
    }

    #[test]
    fn test_resolution_order_reference_must_exist() {
        let json = serde_json::json!({
            "version": "2025.10",
            "resolutionOrder": [
                { "$ref": "#/sets/missing" }
            ]
        });

        let result = Resolver::from_json(&json.to_string());
        assert!(matches!(
            result,
            Err(ResolverError::InvalidReference { .. })
        ));
    }

    #[test]
    fn test_inline_resolution_order_names_must_be_unique() {
        let json = serde_json::json!({
            "version": "2025.10",
            "resolutionOrder": [
                {
                    "type": "set",
                    "name": "theme",
                    "sources": [{ "color": { "bg": srgb_color("#fff", [1.0, 1.0, 1.0]) } }]
                },
                {
                    "type": "modifier",
                    "name": "theme",
                    "contexts": {
                        "light": [{ "color": { "fg": srgb_color("#111", [0.0667, 0.0667, 0.0667]) } }],
                        "dark": [{ "color": { "fg": srgb_color("#eee", [0.9333, 0.9333, 0.9333]) } }]
                    }
                }
            ]
        });

        let result = Resolver::from_json(&json.to_string());
        assert!(matches!(
            result,
            Err(ResolverError::InvalidReference { .. })
        ));
    }

    #[test]
    fn test_set_reference_cycle_is_rejected() {
        let json = serde_json::json!({
            "version": "2025.10",
            "sets": {
                "a": { "sources": [{ "$ref": "#/sets/b" }] },
                "b": { "sources": [{ "$ref": "#/sets/a" }] }
            },
            "resolutionOrder": [
                { "$ref": "#/sets/a" }
            ]
        });

        let result = Resolver::from_json(&json.to_string());
        assert!(matches!(
            result,
            Err(ResolverError::CircularReference { .. })
        ));
    }

    #[test]
    fn test_materializes_aliases_after_merge() {
        let json = serde_json::json!({
            "version": "2025.10",
            "sets": {
                "base": {
                    "sources": [{
                        "foundation": {
                            "colors": {
                                "$type": "color",
                                "primary": {
                                    "$value": {
                                        "colorSpace": "srgb",
                                        "components": [0.2, 0.4, 1.0],
                                        "hex": "#3366ff"
                                    }
                                },
                                "secondary": {
                                    "$value": "{foundation.colors.primary}"
                                }
                            }
                        }
                    }]
                }
            },
            "resolutionOrder": [
                { "$ref": "#/sets/base" }
            ]
        });

        let resolver = Resolver::from_json(&json.to_string()).expect("Failed to create resolver");
        let result = resolver
            .resolve(ResolutionInput::new())
            .expect("Failed to resolve");

        assert_eq!(
            result["foundation"]["colors"]["secondary"]["$value"],
            result["foundation"]["colors"]["primary"]["$value"]
        );
    }

    #[test]
    fn test_materialize_alias_cycle_returns_error() {
        let json = serde_json::json!({
            "version": "2025.10",
            "sets": {
                "base": {
                    "sources": [{
                        "semantic": {
                            "$type": "number",
                            "a": { "$value": "{semantic.b}" },
                            "b": { "$value": "{semantic.a}" }
                        }
                    }]
                }
            },
            "resolutionOrder": [
                { "$ref": "#/sets/base" }
            ]
        });

        let resolver = Resolver::from_json(&json.to_string()).expect("Failed to create resolver");
        let result = resolver.resolve(ResolutionInput::new());

        assert!(matches!(
            result,
            Err(ResolverError::CircularReference { .. })
        ));
    }

    #[test]
    fn test_resolve_with_options_can_preserve_aliases() {
        let json = serde_json::json!({
            "version": "2025.10",
            "sets": {
                "base": {
                    "sources": [{
                        "foundation": {
                            "colors": {
                                "$type": "color",
                                "primary": {
                                    "$value": {
                                        "colorSpace": "srgb",
                                        "components": [0.2, 0.4, 1.0],
                                        "hex": "#3366ff"
                                    }
                                },
                                "secondary": {
                                    "$value": "{foundation.colors.primary}"
                                }
                            }
                        }
                    }]
                }
            },
            "resolutionOrder": [
                { "$ref": "#/sets/base" }
            ]
        });

        let resolver = Resolver::from_json(&json.to_string()).expect("Failed to create resolver");
        let result = resolver
            .resolve_with_options(
                ResolutionInput::new(),
                ResolveOptions {
                    materialize_aliases: false,
                    materialize_property_refs: true,
                    convert_token_refs_to_aliases: false,
                },
            )
            .expect("Failed to resolve with alias preservation");

        assert_eq!(
            result["foundation"]["colors"]["secondary"]["$value"],
            "{foundation.colors.primary}"
        );
    }

    #[test]
    fn test_resolve_with_options_can_convert_token_level_refs_to_aliases() {
        let json = serde_json::json!({
            "version": "2025.10",
            "sets": {
                "base": {
                    "sources": [{
                        "foundation": {
                            "colors": {
                                "$type": "color",
                                "primary": {
                                    "$value": {
                                        "colorSpace": "srgb",
                                        "components": [0.2, 0.4, 1.0],
                                        "hex": "#3366ff"
                                    }
                                },
                                "secondary": {
                                    "$value": {
                                        "$ref": "#/foundation/colors/primary/$value"
                                    }
                                },
                                "accentHex": {
                                    "$value": {
                                        "$ref": "#/foundation/colors/primary/$value/hex"
                                    }
                                }
                            }
                        }
                    }]
                }
            },
            "resolutionOrder": [
                { "$ref": "#/sets/base" }
            ]
        });

        let resolver = Resolver::from_json(&json.to_string()).expect("Failed to create resolver");
        let result = resolver
            .resolve_with_options(
                ResolutionInput::new(),
                ResolveOptions {
                    materialize_aliases: false,
                    materialize_property_refs: false,
                    convert_token_refs_to_aliases: true,
                },
            )
            .expect("Failed to resolve with token ref alias conversion");

        assert_eq!(
            result["foundation"]["colors"]["secondary"]["$value"],
            "{foundation.colors.primary}"
        );

        // Property-level ref targets should remain JSON refs at token root.
        assert_eq!(
            result["foundation"]["colors"]["accentHex"]["$value"]["$ref"],
            "#/foundation/colors/primary/$value/hex"
        );
    }
}
