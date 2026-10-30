//! A resolver with all of its files loaded and parsed, ready to be resolved for any input.
//!
//! `core` never opens files. [`Bundle::load`] asks a [`SourceProvider`] for each file, and a host that cannot
//! read synchronously (Node, WASM) can call [`Bundle::referenced_files`] first, fetch what it names, and pass the
//! results in as [`MemorySources`](super::provider::MemorySources).

use indexmap::IndexMap;
use json_spanned::parser::{JsonValue, Spanned};

use crate::{
    diagnostics::{
        code::DiagnosticCode,
        diagnostic::{Diagnostic, DiagnosticSink, DiagnosticStore},
    },
    ir::{
        merge::LayerOrigin,
        pipeline::{Compiled, Layer, compile},
    },
    resolver::{
        document::{self, OrderItem, ResolverDocument, TokenSource, navigate},
        input::Input,
        provider::{ProviderError, SourceProvider},
    },
    source::{SourceId, SourceMap, SourceSpan},
};

pub struct Bundle {
    document: ResolverDocument,
    resolver_tree: Spanned<JsonValue>,
    files: IndexMap<String, (SourceId, Spanned<JsonValue>)>,
}

fn parse_source(
    id: SourceId,
    text: &str,
    diags: &mut DiagnosticStore,
) -> Option<Spanned<JsonValue>> {
    let (tree, errors) = json_spanned::parse_json(text);
    for e in &errors {
        let span = e.span();
        diags.emit(
            Diagnostic::new(DiagnosticCode::InvalidJsonSyntax)
                .with_message(format!("invalid JSON: {:?}", e.reason()))
                .primary(SourceSpan::new(id, span.start, span.end), "here"),
        );
    }
    if errors.is_empty() { tree } else { None }
}

impl Bundle {
    /// The files a resolver refers to, as paths for [`SourceProvider::read`]. Returns `None` if the resolver itself
    /// is invalid; call [`Bundle::load`] to see why.
    pub fn referenced_files(root_path: &str, root_text: &str) -> Option<Vec<String>> {
        let mut sources = SourceMap::new();
        let mut diags = DiagnosticStore::new();
        let id = sources.register_source(root_path, root_text);
        let tree = parse_source(id, root_text, &mut diags)?;
        let doc = document::parse(id, root_path, &tree, &mut diags)?;
        Some(doc.file_paths().into_iter().map(|(p, _)| p).collect())
    }

    /// Parses the resolver and every file it names. Problems are reported to `diags`; `None` means the bundle
    /// cannot be used.
    pub fn load(
        root_path: &str,
        root_text: &str,
        provider: &dyn SourceProvider,
        sources: &mut SourceMap,
        diags: &mut DiagnosticStore,
    ) -> Option<Self> {
        let id = sources.register_source(root_path, root_text);
        let tree = parse_source(id, root_text, diags)?;
        let document = document::parse(id, root_path, &tree, diags)?;

        let mut files = IndexMap::new();
        let mut ok = true;
        for (path, span) in document.file_paths() {
            let text = match provider.read(&path) {
                Ok(t) => t,
                Err(e) => {
                    ok = false;
                    let (code, msg) = match e {
                        ProviderError::NotFound => (
                            DiagnosticCode::FileNotFound,
                            format!("`{path}` was not found"),
                        ),
                        ProviderError::Unreadable(why) => (
                            DiagnosticCode::FileReadError,
                            format!("`{path}` could not be read: {why}"),
                        ),
                    };
                    diags.emit(
                        Diagnostic::new(code)
                            .with_message(msg)
                            .primary(span, "referenced here"),
                    );
                    continue;
                }
            };
            let file_id = sources.register_source(&path, text.as_str());
            match parse_source(file_id, &text, diags) {
                Some(t) => {
                    files.insert(path, (file_id, t));
                }
                None => ok = false,
            }
        }
        if !ok {
            return None;
        }

        let bundle = Self {
            document,
            resolver_tree: tree,
            files,
        };
        bundle.check_targets(diags).then_some(bundle)
    }

    pub fn document(&self) -> &ResolverDocument {
        &self.document
    }

    fn target(&self, source: &TokenSource) -> Option<&Spanned<JsonValue>> {
        match source {
            TokenSource::Inline { pointer, .. } => navigate(&self.resolver_tree, pointer),
            TokenSource::File { path, pointer, .. } => navigate(&self.files.get(path)?.1, pointer),
        }
    }

    /// Every source must lead to a JSON object, since that is what a token tree is.
    fn check_targets(&self, diags: &mut DiagnosticStore) -> bool {
        let mut ok = true;
        let mut check = |s: &TokenSource, diags: &mut DiagnosticStore| {
            let (span, what) = match s {
                TokenSource::Inline { span, .. } => (span.clone(), "inline tokens"),
                TokenSource::File { span, .. } => (span.clone(), "referenced tokens"),
            };
            match self.target(s).map(|t| &t.value) {
                Some(JsonValue::Object(_)) => {}
                Some(_) => {
                    ok = false;
                    diags.emit(
                        Diagnostic::new(DiagnosticCode::InvalidResolverShape)
                            .with_message(format!("{what} must be a JSON object"))
                            .primary(span, "not an object"),
                    );
                }
                None => {
                    ok = false;
                    diags.emit(
                        Diagnostic::new(DiagnosticCode::InvalidResolverReference)
                            .with_message("the referenced pointer does not exist in the file")
                            .primary(span, "no value at this pointer"),
                    );
                }
            }
        };
        for item in &self.document.order {
            if let OrderItem::Set(set) = item {
                set.sources.iter().for_each(|s| check(s, diags));
            }
        }
        for m in self.document.modifiers.values() {
            m.contexts.values().flatten().for_each(|s| check(s, diags));
        }
        ok
    }

    /// The token layers for `input`, in `resolutionOrder` order (later layers win).
    pub fn layers(&self, input: &Input, diags: &mut DiagnosticStore) -> Option<Vec<Layer<'_>>> {
        let selection = input.select(&self.document, diags)?;
        let mut layers = Vec::new();
        let mut push = |s: &TokenSource| {
            let tree = self.target(s)?;
            layers.push(match s {
                TokenSource::Inline { pointer, .. } => Layer {
                    source: self.document.source,
                    origin: LayerOrigin::Inline {
                        resolver: self.document.source,
                        pointer: pointer.clone(),
                    },
                    tree,
                },
                TokenSource::File { path, .. } => {
                    let id = self.files[path].0;
                    Layer {
                        source: id,
                        origin: LayerOrigin::File(id),
                        tree,
                    }
                }
            });
            Some(())
        };
        for item in &self.document.order {
            match item {
                OrderItem::Set(set) => set.sources.iter().try_for_each(&mut push)?,
                OrderItem::Modifier(name) => self.document.modifiers[name].contexts
                    [&selection[name]]
                    .iter()
                    .try_for_each(&mut push)?,
            }
        }
        Some(layers)
    }

    /// Resolves the bundle for `input` all the way to compiled tokens.
    pub fn compile(&self, input: &Input, diags: &mut DiagnosticStore) -> Option<Compiled> {
        let layers = self.layers(input, diags)?;
        compile(&layers, diags).ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::resolver::provider::MemorySources;

    const RESOLVER: &str = r##"{
        "version": "2025.10",
        "sets": {"base": {"sources": [{"$ref": "base.json"}]}},
        "modifiers": {"theme": {"default": "light", "contexts": {
            "light": [{"$ref": "light.json"}],
            "dark": [{"$ref": "themes.json#/dark"}]}}},
        "resolutionOrder": [
            {"$ref": "#/sets/base"},
            {"$ref": "#/modifiers/theme"},
            {"name": "overrides", "type": "set", "sources": [
                {"color": {"accent": {"$type": "color", "$value": {"colorSpace": "srgb", "components": [1, 0, 0]}}}}
            ]}
        ]
    }"##;

    fn files() -> MemorySources {
        let mut m = MemorySources::new();
        m.insert(
            "tokens/base.json",
            r##"{"color": {"$type": "color",
                "bg": {"$value": {"colorSpace": "srgb", "components": [1, 1, 1]}},
                "fg": {"$value": {"colorSpace": "srgb", "components": [0, 0, 0]}}}}"##,
        );
        m.insert(
            "tokens/light.json",
            r##"{"color": {"fg": {"$value": {"colorSpace": "srgb", "components": [0.1, 0.1, 0.1]}}}}"##,
        );
        m.insert(
            "tokens/themes.json",
            r##"{"dark": {"color": {"bg": {"$value": {"colorSpace": "srgb", "components": [0, 0, 0]}},
                "fg": {"$value": "{color.accent}"}}}}"##,
        );
        m
    }

    fn load(files: &MemorySources) -> (Option<Bundle>, DiagnosticStore) {
        let mut d = DiagnosticStore::new();
        let b = Bundle::load(
            "tokens/resolver.json",
            RESOLVER,
            files,
            &mut SourceMap::new(),
            &mut d,
        );
        (b, d)
    }

    fn components(c: &Compiled, path: &str) -> serde_json::Value {
        let json = serde_json::to_value(c.resolved()).unwrap();
        let token = json
            .as_array()
            .unwrap()
            .iter()
            .find(|t| t["path"] == serde_json::Value::from(path.split('.').collect::<Vec<_>>()))
            .unwrap_or_else(|| panic!("no token {path}: {json}"));
        token["value"]["value"]["components"].clone()
    }

    #[test]
    fn lists_referenced_files() {
        assert_eq!(
            Bundle::referenced_files("tokens/resolver.json", RESOLVER).unwrap(),
            [
                "tokens/base.json",
                "tokens/light.json",
                "tokens/themes.json"
            ]
        );
    }

    #[test]
    fn resolves_default_and_selected_contexts() {
        let (b, d) = load(&files());
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
        let b = b.unwrap();
        let mut d = DiagnosticStore::new();

        let light = b.compile(&Input::new(), &mut d).unwrap();
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
        assert_eq!(
            components(&light, "color.fg"),
            serde_json::json!([0.1, 0.1, 0.1])
        );
        assert_eq!(
            components(&light, "color.bg"),
            serde_json::json!([1.0, 1.0, 1.0])
        );

        let dark = b
            .compile(&Input::new().with("theme", "dark"), &mut d)
            .unwrap();
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
        assert_eq!(
            components(&dark, "color.bg"),
            serde_json::json!([0.0, 0.0, 0.0])
        );
        // The alias in the dark context resolves against the later inline set.
        assert_eq!(
            components(&dark, "color.fg"),
            serde_json::json!([1.0, 0.0, 0.0])
        );
    }

    #[test]
    fn missing_file_is_reported_at_the_reference() {
        let mut m = MemorySources::new();
        m.insert("tokens/base.json", "{}");
        m.insert("tokens/themes.json", "{\"dark\": {}}");
        let (b, d) = load(&m);
        assert!(b.is_none());
        let diag = &d.diagnostics()[0];
        assert_eq!(diag.code, DiagnosticCode::FileNotFound);
        assert_eq!(diag.labels.len(), 1);
    }

    #[test]
    fn bad_json_in_a_file_is_reported() {
        let mut m = files();
        m.insert("tokens/base.json", "{ nope");
        let (b, d) = load(&m);
        assert!(b.is_none());
        assert!(
            d.diagnostics()
                .iter()
                .any(|d| d.code == DiagnosticCode::InvalidJsonSyntax)
        );
    }

    #[test]
    fn fragment_must_exist_and_be_an_object() {
        let mut m = files();
        m.insert("tokens/themes.json", r#"{"dark": 5}"#);
        let (b, d) = load(&m);
        assert!(b.is_none());
        assert_eq!(
            d.diagnostics()[0].code,
            DiagnosticCode::InvalidResolverShape
        );

        m.insert("tokens/themes.json", r#"{"other": {}}"#);
        let (b, d) = load(&m);
        assert!(b.is_none());
        assert_eq!(
            d.diagnostics()[0].code,
            DiagnosticCode::InvalidResolverReference
        );
    }

    #[test]
    fn invalid_input_stops_before_compiling() {
        let (b, _) = load(&files());
        let mut d = DiagnosticStore::new();
        assert!(
            b.unwrap()
                .compile(&Input::new().with("theme", "neon"), &mut d)
                .is_none()
        );
        assert_eq!(
            d.diagnostics()[0].code,
            DiagnosticCode::InvalidResolverInput
        );
    }
}
