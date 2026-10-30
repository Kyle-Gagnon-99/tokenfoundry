//! The resolver document: parsing, static validation and normalization.
//!
//! This pass never reads files. It turns the resolver's JSON into a [`ResolverDocument`] in which every set and
//! modifier context is a flat list of [`TokenSource`]s, with `#/sets/...` and same-document `$ref`s already
//! expanded. Files it needs are only *named* (see [`ResolverDocument::file_paths`]), so a host can load them
//! however it likes, including asynchronously.

use std::collections::{HashMap, HashSet};

use indexmap::IndexMap;
use json_spanned::parser::{JsonMember, JsonValue, Span, Spanned};

use crate::{
    ast::reference::{DocumentReference, JsonPointer, JsonReference, JsonReferenceParseError},
    diagnostics::{
        code::DiagnosticCode,
        diagnostic::{Diagnostic, DiagnosticSink, DiagnosticStore},
    },
    resolver::provider,
    source::{SourceId, SourceSpan},
};

pub const RESOLVER_VERSION: &str = "2025.10";

/// Where one layer of tokens comes from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TokenSource {
    /// Tokens written inside the resolver document, at `pointer`.
    Inline {
        pointer: JsonPointer,
        span: SourceSpan,
    },
    /// Tokens in another file, at `pointer` (empty for the whole file). `path` is joined to the resolver's
    /// location and normalized.
    File {
        path: String,
        pointer: JsonPointer,
        span: SourceSpan,
    },
}

#[derive(Debug, Clone)]
pub struct Set {
    pub name: String,
    pub description: Option<String>,
    pub sources: Vec<TokenSource>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub struct Modifier {
    pub name: String,
    pub description: Option<String>,
    pub contexts: IndexMap<String, Vec<TokenSource>>,
    pub default: Option<String>,
    pub span: SourceSpan,
}

#[derive(Debug, Clone)]
pub enum OrderItem {
    Set(Set),
    /// The modifier's name; look it up in [`ResolverDocument::modifiers`].
    Modifier(String),
}

#[derive(Debug, Clone)]
pub struct ResolverDocument {
    pub name: Option<String>,
    pub description: Option<String>,
    pub source: SourceId,
    /// Every modifier, declared in `modifiers` or inline in `resolutionOrder`.
    pub modifiers: IndexMap<String, Modifier>,
    pub order: Vec<OrderItem>,
}

impl ResolverDocument {
    /// Every external file the document refers to, once each, with the first place that mentions it.
    pub fn file_paths(&self) -> Vec<(String, SourceSpan)> {
        let mut seen = HashSet::new();
        let mut out = Vec::new();
        let mut visit = |sources: &[TokenSource]| {
            for s in sources {
                if let TokenSource::File { path, span, .. } = s
                    && seen.insert(path.clone())
                {
                    out.push((path.clone(), span.clone()));
                }
            }
        };
        for item in &self.order {
            match item {
                OrderItem::Set(set) => visit(&set.sources),
                OrderItem::Modifier(name) => {
                    for sources in self.modifiers[name].contexts.values() {
                        visit(sources);
                    }
                }
            }
        }
        for (name, modifier) in &self.modifiers {
            let in_order = self
                .order
                .iter()
                .any(|i| matches!(i, OrderItem::Modifier(n) if n == name));
            if !in_order {
                for sources in modifier.contexts.values() {
                    visit(sources);
                }
            }
        }
        out
    }
}

/// Follows a JSON pointer through a parsed document.
pub fn navigate<'a>(
    root: &'a Spanned<JsonValue>,
    pointer: &JsonPointer,
) -> Option<&'a Spanned<JsonValue>> {
    let mut cur = root;
    for seg in pointer.segments() {
        cur = match &cur.value {
            JsonValue::Object(members) => &members.iter().find(|m| m.key.value == *seg)?.value,
            JsonValue::Array(items) => items.get(seg.parse::<usize>().ok()?)?,
            _ => return None,
        };
    }
    Some(cur)
}

/// Parses and validates a resolver document. `doc_path` is the resolver's own path, which relative file references
/// are joined to. Returns `None` if anything was reported as an error.
pub fn parse(
    source: SourceId,
    doc_path: &str,
    tree: &Spanned<JsonValue>,
    diags: &mut DiagnosticStore,
) -> Option<ResolverDocument> {
    let errors_before = diags.error_count();
    let doc = Builder::new(source, doc_path, tree, diags).run();
    if diags.error_count() > errors_before {
        None
    } else {
        doc
    }
}

type Member<'a> = (&'a JsonMember, JsonPointer);

struct Builder<'a, 'd> {
    source: SourceId,
    doc_path: &'d str,
    root: &'a Spanned<JsonValue>,
    diags: &'d mut DiagnosticStore,
    raw_sets: IndexMap<String, &'a JsonMember>,
    raw_modifiers: IndexMap<String, &'a JsonMember>,
    set_cache: HashMap<String, Option<Set>>,
    set_stack: Vec<String>,
    ref_stack: Vec<String>,
}

impl<'a, 'd> Builder<'a, 'd> {
    fn new(
        source: SourceId,
        doc_path: &'d str,
        root: &'a Spanned<JsonValue>,
        diags: &'d mut DiagnosticStore,
    ) -> Self {
        Self {
            source,
            doc_path,
            root,
            diags,
            raw_sets: IndexMap::new(),
            raw_modifiers: IndexMap::new(),
            set_cache: HashMap::new(),
            set_stack: Vec::new(),
            ref_stack: Vec::new(),
        }
    }

    fn span(&self, span: Span) -> SourceSpan {
        SourceSpan::from_json_span(self.source, span)
    }

    fn error(&mut self, code: DiagnosticCode, span: Span, message: impl Into<String>, label: &str) {
        let span = self.span(span);
        self.diags.emit(
            Diagnostic::new(code)
                .with_message(message)
                .primary(span, label),
        );
    }

    fn shape(&mut self, span: Span, message: impl Into<String>, label: &str) {
        self.error(DiagnosticCode::InvalidResolverShape, span, message, label);
    }

    fn check_duplicate_keys(&mut self, value: &Spanned<JsonValue>) {
        match &value.value {
            JsonValue::Object(members) => {
                let mut seen: HashMap<&str, ()> = HashMap::new();
                for m in members {
                    if seen.insert(m.key.value.as_str(), ()).is_some() {
                        self.error(
                            DiagnosticCode::DuplicateKeyInJson,
                            m.key.span,
                            format!("duplicate key `{}`", m.key.value),
                            "this key already appeared in the same object",
                        );
                    }
                    self.check_duplicate_keys(&m.value);
                }
            }
            JsonValue::Array(items) => items.iter().for_each(|i| self.check_duplicate_keys(i)),
            _ => {}
        }
    }

    fn string_member(&mut self, m: &JsonMember) -> Option<String> {
        match &m.value.value {
            JsonValue::String(s) => Some(s.clone()),
            _ => {
                self.shape(
                    m.value.span,
                    format!("`{}` must be a string", m.key.value),
                    "expected a string",
                );
                None
            }
        }
    }

    fn run(mut self) -> Option<ResolverDocument> {
        let root = self.root;
        self.check_duplicate_keys(root);
        let JsonValue::Object(members) = &root.value else {
            self.shape(
                root.span,
                "a resolver document must be a JSON object",
                "expected an object",
            );
            return None;
        };

        let mut name = None;
        let mut description = None;
        let mut order_member = None;
        let mut sets_member = None;
        let mut modifiers_member = None;
        for m in members {
            match m.key.value.as_str() {
                "$schema" | "$defs" => {}
                "name" => name = self.string_member(m),
                "description" => description = self.string_member(m),
                "version" => match &m.value.value {
                    JsonValue::String(v) if v == RESOLVER_VERSION => {}
                    _ => self.shape(
                        m.value.span,
                        format!("`version` must be \"{RESOLVER_VERSION}\""),
                        "unsupported resolver version",
                    ),
                },
                "sets" => sets_member = Some(m),
                "modifiers" => modifiers_member = Some(m),
                "resolutionOrder" => order_member = Some(m),
                other => self.shape(
                    m.key.span,
                    format!("unknown top-level property `{other}`"),
                    "not part of the resolver format",
                ),
            }
        }
        if !members.iter().any(|m| m.key.value == "version") {
            self.shape(
                root.span,
                "missing required `version`",
                "add \"version\": \"2025.10\"",
            );
        }

        if let Some(m) = sets_member {
            self.collect_named(m, true);
        }
        if let Some(m) = modifiers_member {
            self.collect_named(m, false);
        }
        let set_keys: Vec<String> = self.raw_sets.keys().cloned().collect();
        for name in &set_keys {
            if let Some(m) = self.raw_modifiers.get(name) {
                let span = m.key.span;
                let message = format!("`{name}` is declared as both a set and a modifier");
                self.error(
                    DiagnosticCode::DuplicateResolverName,
                    span,
                    message,
                    "names must be unique",
                );
            }
        }

        let mut modifiers = IndexMap::new();
        let set_names: Vec<String> = self.raw_sets.keys().cloned().collect();
        for n in &set_names {
            self.declared_set(n);
        }
        let modifier_names: Vec<String> = self.raw_modifiers.keys().cloned().collect();
        for n in &modifier_names {
            let raw = self.raw_modifiers[n];
            let ptr = JsonPointer::from_segments(["modifiers", n.as_str()]);
            let Some(members) = self.object_members(&raw.value, &ptr) else {
                continue;
            };
            if let Some(m) = self.build_modifier(n, &members, raw.value.span) {
                modifiers.insert(n.clone(), m);
            }
        }

        let order_member = match order_member {
            Some(m) => m,
            None => {
                self.shape(
                    root.span,
                    "missing required `resolutionOrder`",
                    "add a `resolutionOrder` array",
                );
                return None;
            }
        };
        let order = self.build_order(order_member, &mut modifiers)?;

        Some(ResolverDocument {
            name,
            description,
            source: self.source,
            modifiers,
            order,
        })
    }

    fn collect_named(&mut self, member: &'a JsonMember, sets: bool) {
        let JsonValue::Object(entries) = &member.value.value else {
            self.shape(
                member.value.span,
                format!("`{}` must be an object", member.key.value),
                "expected an object",
            );
            return;
        };
        for e in entries {
            let map = if sets {
                &mut self.raw_sets
            } else {
                &mut self.raw_modifiers
            };
            map.insert(e.key.value.clone(), e);
        }
    }

    /// The members of an object value, each with the pointer of the object that holds them.
    fn object_members(
        &mut self,
        value: &'a Spanned<JsonValue>,
        ptr: &JsonPointer,
    ) -> Option<Vec<Member<'a>>> {
        match &value.value {
            JsonValue::Object(ms) => Some(ms.iter().map(|m| (m, ptr.clone())).collect()),
            _ => {
                self.shape(value.span, "expected an object", "expected an object");
                None
            }
        }
    }

    fn declared_set(&mut self, name: &str) -> Option<Set> {
        if let Some(cached) = self.set_cache.get(name) {
            return cached.clone();
        }
        let raw = *self.raw_sets.get(name)?;
        if self.set_stack.iter().any(|n| n == name) {
            self.error(
                DiagnosticCode::ResolverReferenceCycle,
                raw.key.span,
                format!("set `{name}` includes itself"),
                "this set is part of a reference cycle",
            );
            return None;
        }
        self.set_stack.push(name.to_string());
        let ptr = JsonPointer::from_segments(["sets", name]);
        let built = self
            .object_members(&raw.value, &ptr)
            .and_then(|ms| self.build_set(name, &ms, raw.value.span));
        self.set_stack.pop();
        self.set_cache.insert(name.to_string(), built.clone());
        built
    }

    fn build_set(&mut self, name: &str, members: &[Member<'a>], span: Span) -> Option<Set> {
        let mut description = None;
        let mut sources = None;
        for (m, parent) in members {
            match m.key.value.as_str() {
                "description" => description = self.string_member(m),
                "$extensions" | "name" | "type" => {}
                "sources" => {
                    let mut ptr = parent.clone();
                    ptr.append("sources".into());
                    sources = self.expand_list(&m.value, ptr);
                }
                other => self.shape(
                    m.key.span,
                    format!("unknown property `{other}` on set `{name}`"),
                    "sets only accept `description`, `sources` and `$extensions`",
                ),
            }
        }
        let Some(sources) = sources else {
            if !members.iter().any(|(m, _)| m.key.value == "sources") {
                self.shape(
                    span,
                    format!("set `{name}` is missing `sources`"),
                    "add a `sources` array",
                );
            }
            return None;
        };
        Some(Set {
            name: name.to_string(),
            description,
            sources,
            span: self.span(span),
        })
    }

    fn build_modifier(
        &mut self,
        name: &str,
        members: &[Member<'a>],
        span: Span,
    ) -> Option<Modifier> {
        let mut description = None;
        let mut contexts = None;
        let mut default = None;
        let mut default_span = span;
        for (m, parent) in members {
            match m.key.value.as_str() {
                "description" => description = self.string_member(m),
                "$extensions" | "name" | "type" => {}
                "default" => {
                    default_span = m.value.span;
                    default = self.string_member(m);
                }
                "contexts" => {
                    let mut ptr = parent.clone();
                    ptr.append("contexts".into());
                    contexts = self.build_contexts(name, &m.value, ptr);
                }
                other => self.shape(
                    m.key.span,
                    format!("unknown property `{other}` on modifier `{name}`"),
                    "modifiers only accept `description`, `contexts`, `default` and `$extensions`",
                ),
            }
        }
        let Some(contexts) = contexts else {
            if !members.iter().any(|(m, _)| m.key.value == "contexts") {
                self.shape(
                    span,
                    format!("modifier `{name}` is missing `contexts`"),
                    "add a `contexts` object",
                );
            }
            return None;
        };
        if let Some(d) = &default
            && !contexts.contains_key(d)
        {
            self.error(
                DiagnosticCode::InvalidModifier,
                default_span,
                format!("default `{d}` is not a context of modifier `{name}`"),
                "must be one of the keys of `contexts`",
            );
            return None;
        }
        Some(Modifier {
            name: name.to_string(),
            description,
            contexts,
            default,
            span: self.span(span),
        })
    }

    fn build_contexts(
        &mut self,
        modifier: &str,
        value: &'a Spanned<JsonValue>,
        ptr: JsonPointer,
    ) -> Option<IndexMap<String, Vec<TokenSource>>> {
        let JsonValue::Object(entries) = &value.value else {
            self.shape(
                value.span,
                "`contexts` must be an object",
                "expected an object",
            );
            return None;
        };
        if entries.len() < 2 {
            self.error(
                DiagnosticCode::InvalidModifier,
                value.span,
                format!(
                    "modifier `{modifier}` needs at least two contexts, found {}",
                    entries.len()
                ),
                "a modifier with fewer than two contexts has nothing to choose between",
            );
            return None;
        }
        let mut out = IndexMap::new();
        let mut ok = true;
        for e in entries {
            let mut p = ptr.clone();
            p.append(e.key.value.clone());
            match self.expand_list(&e.value, p) {
                Some(sources) => {
                    out.insert(e.key.value.clone(), sources);
                }
                None => ok = false,
            }
        }
        ok.then_some(out)
    }

    fn expand_list(
        &mut self,
        value: &'a Spanned<JsonValue>,
        ptr: JsonPointer,
    ) -> Option<Vec<TokenSource>> {
        let JsonValue::Array(items) = &value.value else {
            self.shape(
                value.span,
                "expected an array of token sources",
                "expected an array",
            );
            return None;
        };
        let errors = self.diags.error_count();
        let mut out = Vec::new();
        for (i, item) in items.iter().enumerate() {
            let mut p = ptr.clone();
            p.append(i.to_string());
            self.expand_item(item, p, &mut out);
        }
        (self.diags.error_count() == errors).then_some(out)
    }

    fn reference(&mut self, value: &Spanned<JsonValue>) -> Option<JsonReference> {
        let JsonValue::String(raw) = &value.value else {
            self.error(
                DiagnosticCode::InvalidResolverReference,
                value.span,
                "`$ref` must be a string",
                "expected a string",
            );
            return None;
        };
        match JsonReference::from_str(raw) {
            Ok(r) => Some(r),
            Err(e) => {
                let span = self.span(value.span);
                let code = match e {
                    JsonReferenceParseError::UnsupportedScheme { .. } => {
                        DiagnosticCode::UnsupportedResolverReference
                    }
                    _ => DiagnosticCode::InvalidResolverReference,
                };
                let mut d = Diagnostic::new(code)
                    .with_message(format!("invalid reference `{raw}`"))
                    .primary(span, "could not be parsed");
                if let Some(hint) = e.hint() {
                    d = d.help(hint);
                }
                self.diags.emit(d);
                None
            }
        }
    }

    fn expand_item(
        &mut self,
        item: &'a Spanned<JsonValue>,
        ptr: JsonPointer,
        out: &mut Vec<TokenSource>,
    ) {
        let JsonValue::Object(members) = &item.value else {
            self.shape(
                item.span,
                "a token source must be an object",
                "expected an object, either a `$ref` or inline tokens",
            );
            return;
        };
        let Some(ref_member) = members.iter().find(|m| m.key.value == "$ref") else {
            out.push(TokenSource::Inline {
                pointer: ptr,
                span: self.span(item.span),
            });
            return;
        };
        if let Some(extra) = members.iter().find(|m| m.key.value != "$ref") {
            self.error(
                DiagnosticCode::UnsupportedResolverReference,
                extra.key.span,
                "properties next to a `$ref` in a token source are not supported",
                "move this into the referenced object",
            );
            return;
        }
        let Some(reference) = self.reference(&ref_member.value) else {
            return;
        };
        let span = ref_member.value.span;
        let fragment = reference.fragment().clone();

        match reference.document() {
            Some(DocumentReference::RelativeFile(path)) => out.push(TokenSource::File {
                path: provider::join(self.doc_path, path),
                pointer: fragment,
                span: self.span(span),
            }),
            Some(DocumentReference::UnsupportedScheme { scheme, .. }) => {
                let message = format!("`{scheme}:` references are not supported");
                self.error(
                    DiagnosticCode::UnsupportedResolverReference,
                    span,
                    message,
                    "only relative file paths are supported",
                );
            }
            None => self.expand_local(fragment, ptr, span, out),
        }
    }

    fn expand_local(
        &mut self,
        fragment: JsonPointer,
        from: JsonPointer,
        span: Span,
        out: &mut Vec<TokenSource>,
    ) {
        let segs = fragment.segments();
        match segs.first().map(String::as_str) {
            None => {
                self.error(
                    DiagnosticCode::ResolverReferenceCycle,
                    span,
                    "a reference to the document root includes itself",
                    "reference a set or a token group instead",
                );
            }
            Some("modifiers" | "resolutionOrder") => {
                self.error(
                    DiagnosticCode::InvalidResolverReference,
                    span,
                    format!(
                        "`#{}` cannot be referenced here",
                        fragment.to_pointer_string()
                    ),
                    "only `#/sets/...` and inline token data may be referenced from sources",
                );
            }
            Some("sets") if segs.len() == 2 => {
                let name = segs[1].clone();
                if !self.raw_sets.contains_key(&name) {
                    self.error(
                        DiagnosticCode::InvalidResolverReference,
                        span,
                        format!("there is no set named `{name}`"),
                        "unknown set",
                    );
                } else if let Some(set) = self.declared_set(&name) {
                    out.extend(set.sources);
                } else if !self.set_cache.contains_key(&name) {
                    // A cycle was already reported while the set was being built.
                }
            }
            Some(_) => {
                let root = self.root;
                let Some(target) = navigate(root, &fragment) else {
                    self.error(
                        DiagnosticCode::InvalidResolverReference,
                        span,
                        format!(
                            "`#{}` does not exist in this document",
                            fragment.to_pointer_string()
                        ),
                        "no value at this pointer",
                    );
                    return;
                };
                let key = fragment.to_pointer_string();
                let is_ancestor = from.segments().starts_with(fragment.segments());
                if is_ancestor || self.ref_stack.contains(&key) {
                    self.error(
                        DiagnosticCode::ResolverReferenceCycle,
                        span,
                        format!("`#{key}` leads back to itself"),
                        "this reference is part of a cycle",
                    );
                    return;
                }
                self.ref_stack.push(key);
                self.expand_item(target, fragment, out);
                self.ref_stack.pop();
            }
        }
    }

    fn build_order(
        &mut self,
        member: &'a JsonMember,
        modifiers: &mut IndexMap<String, Modifier>,
    ) -> Option<Vec<OrderItem>> {
        let JsonValue::Array(items) = &member.value.value else {
            self.shape(
                member.value.span,
                "`resolutionOrder` must be an array",
                "expected an array",
            );
            return None;
        };
        if items.is_empty() {
            self.shape(
                member.value.span,
                "`resolutionOrder` must not be empty",
                "add at least one set or modifier",
            );
            return None;
        }

        let mut seen: HashMap<String, Span> = HashMap::new();
        let mut out = Vec::new();
        for (i, item) in items.iter().enumerate() {
            let ptr = JsonPointer::from_segments(["resolutionOrder".to_string(), i.to_string()]);
            let JsonValue::Object(ms) = &item.value else {
                self.shape(
                    item.span,
                    "a `resolutionOrder` item must be an object",
                    "expected an object",
                );
                continue;
            };

            let built = if let Some(r) = ms.iter().find(|m| m.key.value == "$ref") {
                self.order_ref(r, ms, &ptr, item.span, modifiers)
            } else {
                self.order_inline(ms, &ptr, item.span, modifiers)
            };
            let Some(entry) = built else { continue };
            let name = match &entry {
                OrderItem::Set(s) => s.name.clone(),
                OrderItem::Modifier(n) => n.clone(),
            };
            if let Some(first) = seen.get(&name) {
                let first = self.span(*first);
                let span = self.span(item.span);
                self.diags.emit(
                    Diagnostic::new(DiagnosticCode::DuplicateResolverName)
                        .with_message(format!("`{name}` appears twice in `resolutionOrder`"))
                        .primary(span, "duplicate")
                        .secondary(first, "first used here"),
                );
                continue;
            }
            seen.insert(name, item.span);
            out.push(entry);
        }
        Some(out)
    }

    fn order_ref(
        &mut self,
        ref_member: &'a JsonMember,
        all: &'a [JsonMember],
        ptr: &JsonPointer,
        span: Span,
        modifiers: &mut IndexMap<String, Modifier>,
    ) -> Option<OrderItem> {
        let reference = self.reference(&ref_member.value)?;
        let rspan = ref_member.value.span;
        if reference.document().is_some() {
            self.error(
                DiagnosticCode::UnsupportedResolverReference,
                rspan,
                "`resolutionOrder` can only reference sets and modifiers in this document",
                "use `#/sets/<name>` or `#/modifiers/<name>`",
            );
            return None;
        }
        let segs = reference.fragment().segments();
        let (kind, name) = match segs {
            [k, n] if k == "sets" || k == "modifiers" => (k.as_str(), n.clone()),
            _ => {
                self.error(
                    DiagnosticCode::InvalidResolverReference,
                    rspan,
                    "a `resolutionOrder` reference must be `#/sets/<name>` or `#/modifiers/<name>`",
                    "invalid target",
                );
                return None;
            }
        };
        let overrides: Vec<Member<'a>> = all
            .iter()
            .filter(|m| m.key.value != "$ref")
            .map(|m| (m, ptr.clone()))
            .collect();

        if kind == "sets" {
            let Some(raw) = self.raw_sets.get(&name).copied() else {
                self.error(
                    DiagnosticCode::InvalidResolverReference,
                    rspan,
                    format!("there is no set named `{name}`"),
                    "unknown set",
                );
                return None;
            };
            if overrides.is_empty() {
                return self.declared_set(&name).map(OrderItem::Set);
            }
            let base = JsonPointer::from_segments(["sets", name.as_str()]);
            let members = self.merged(raw, &base, overrides)?;
            return self.build_set(&name, &members, span).map(OrderItem::Set);
        }

        let Some(raw) = self.raw_modifiers.get(&name).copied() else {
            self.error(
                DiagnosticCode::InvalidResolverReference,
                rspan,
                format!("there is no modifier named `{name}`"),
                "unknown modifier",
            );
            return None;
        };
        if !overrides.is_empty() {
            let base = JsonPointer::from_segments(["modifiers", name.as_str()]);
            let members = self.merged(raw, &base, overrides)?;
            let m = self.build_modifier(&name, &members, span)?;
            modifiers.insert(name.clone(), m);
        } else if !modifiers.contains_key(&name) {
            return None;
        }
        Some(OrderItem::Modifier(name))
    }

    /// The target's members with the sibling members of the `$ref` replacing any with the same key.
    fn merged(
        &mut self,
        raw: &'a JsonMember,
        base: &JsonPointer,
        overrides: Vec<Member<'a>>,
    ) -> Option<Vec<Member<'a>>> {
        let mut members = self.object_members(&raw.value, base)?;
        members.retain(|(m, _)| !overrides.iter().any(|(o, _)| o.key.value == m.key.value));
        members.extend(overrides);
        Some(members)
    }

    fn order_inline(
        &mut self,
        ms: &'a [JsonMember],
        ptr: &JsonPointer,
        span: Span,
        modifiers: &mut IndexMap<String, Modifier>,
    ) -> Option<OrderItem> {
        let name = match ms.iter().find(|m| m.key.value == "name") {
            Some(m) => self.string_member(m)?,
            None => {
                self.shape(
                    span,
                    "an inline `resolutionOrder` item needs a `name`",
                    "add a `name`",
                );
                return None;
            }
        };
        let kind = match ms.iter().find(|m| m.key.value == "type") {
            Some(m) => self.string_member(m)?,
            None => {
                self.shape(
                    span,
                    format!("inline item `{name}` needs a `type` of \"set\" or \"modifier\""),
                    "add a `type`",
                );
                return None;
            }
        };
        if self.raw_sets.contains_key(&name) || self.raw_modifiers.contains_key(&name) {
            self.error(
                DiagnosticCode::DuplicateResolverName,
                span,
                format!("inline item `{name}` has the same name as a declared set or modifier"),
                "names must be unique",
            );
            return None;
        }
        let members: Vec<Member<'a>> = ms.iter().map(|m| (m, ptr.clone())).collect();
        match kind.as_str() {
            "set" => self.build_set(&name, &members, span).map(OrderItem::Set),
            "modifier" => {
                let m = self.build_modifier(&name, &members, span)?;
                modifiers.insert(name.clone(), m);
                Some(OrderItem::Modifier(name))
            }
            other => {
                self.shape(
                    span,
                    format!("unknown item type `{other}`"),
                    "`type` must be \"set\" or \"modifier\"",
                );
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceIdGenerator;

    fn run(text: &str) -> (Option<ResolverDocument>, DiagnosticStore) {
        let (tree, errors) = json_spanned::parse_json(text);
        assert!(errors.is_empty());
        let tree = tree.unwrap();
        let mut diags = DiagnosticStore::new();
        let id = SourceIdGenerator::default().generate();
        let doc = parse(id, "tokens/resolver.json", &tree, &mut diags);
        (doc, diags)
    }

    fn codes(d: &DiagnosticStore) -> Vec<DiagnosticCode> {
        d.diagnostics().iter().map(|d| d.code).collect()
    }

    fn ok(text: &str) -> ResolverDocument {
        let (doc, d) = run(text);
        assert!(!d.has_errors(), "{:?}", d.diagnostics());
        doc.unwrap()
    }

    fn fails(text: &str, code: DiagnosticCode) {
        let (doc, d) = run(text);
        assert!(doc.is_none(), "expected failure");
        assert!(codes(&d).contains(&code), "{:?}", codes(&d));
    }

    #[test]
    fn parses_sets_modifiers_and_order() {
        let doc = ok(r##"{
            "name": "x", "version": "2025.10",
            "sets": {"base": {"sources": [{"$ref": "base.json"}, {"color": {}}]}},
            "modifiers": {"theme": {"default": "light", "contexts": {
                "light": [{"$ref": "../light.json#/colors"}], "dark": [{"$ref": "#/sets/base"}]}}},
            "resolutionOrder": [{"$ref": "#/sets/base"}, {"$ref": "#/modifiers/theme"}]
        }"##);
        assert_eq!(doc.name.as_deref(), Some("x"));
        let OrderItem::Set(base) = &doc.order[0] else {
            panic!()
        };
        assert_eq!(base.sources.len(), 2);
        assert!(
            matches!(&base.sources[0], TokenSource::File { path, .. } if path == "tokens/base.json")
        );
        assert!(
            matches!(&base.sources[1], TokenSource::Inline { pointer, .. }
            if pointer.to_pointer_string() == "/sets/base/sources/1")
        );
        let theme = &doc.modifiers["theme"];
        assert!(
            matches!(&theme.contexts["light"][0], TokenSource::File { path, pointer, .. }
            if path == "light.json" && pointer.to_pointer_string() == "/colors")
        );
        // The context referencing a set gets that set's sources.
        assert_eq!(theme.contexts["dark"].len(), 2);
        let paths: Vec<_> = doc.file_paths().into_iter().map(|(p, _)| p).collect();
        assert_eq!(paths, ["tokens/base.json", "light.json"]);
    }

    #[test]
    fn inline_order_items_and_overrides() {
        let doc = ok(r##"{
            "version": "2025.10",
            "sets": {"base": {"description": "d", "sources": [{"a": {}}]}},
            "resolutionOrder": [
                {"$ref": "#/sets/base", "sources": [{"b": {}}]},
                {"name": "extra", "type": "set", "sources": [{"c": {}}]},
                {"name": "mode", "type": "modifier", "contexts": {"a": [], "b": []}}
            ]
        }"##);
        let OrderItem::Set(s) = &doc.order[0] else {
            panic!()
        };
        assert_eq!(s.description.as_deref(), Some("d"));
        assert!(matches!(&s.sources[0], TokenSource::Inline { pointer, .. }
            if pointer.to_pointer_string() == "/resolutionOrder/0/sources/0"));
        assert!(matches!(&doc.order[2], OrderItem::Modifier(n) if n == "mode"));
        assert!(doc.modifiers.contains_key("mode"));
    }

    #[test]
    fn rejects_bad_top_level() {
        fails(
            r#"{"version": "2025.10"}"#,
            DiagnosticCode::InvalidResolverShape,
        );
        fails(
            r#"{"version": "2024", "resolutionOrder": [{"name":"a","type":"set","sources":[]}]}"#,
            DiagnosticCode::InvalidResolverShape,
        );
        fails(
            r#"{"resolutionOrder": [{"name":"a","type":"set","sources":[]}]}"#,
            DiagnosticCode::InvalidResolverShape,
        );
        fails(
            r#"{"version": "2025.10", "resolutionOrder": []}"#,
            DiagnosticCode::InvalidResolverShape,
        );
        fails(
            r#"{"version": "2025.10", "extra": 1, "resolutionOrder": [{"name":"a","type":"set","sources":[]}]}"#,
            DiagnosticCode::InvalidResolverShape,
        );
    }

    #[test]
    fn rejects_duplicate_names_and_keys() {
        fails(
            r##"{"version":"2025.10","sets":{"a":{"sources":[]}},
            "resolutionOrder":[{"$ref":"#/sets/a"},{"$ref":"#/sets/a"}]}"##,
            DiagnosticCode::DuplicateResolverName,
        );
        fails(
            r##"{"version":"2025.10","sets":{"a":{"sources":[]}},
            "resolutionOrder":[{"name":"a","type":"set","sources":[]}]}"##,
            DiagnosticCode::DuplicateResolverName,
        );
        fails(
            r#"{"version":"2025.10","version":"2025.10","resolutionOrder":[{"name":"a","type":"set","sources":[]}]}"#,
            DiagnosticCode::DuplicateKeyInJson,
        );
    }

    #[test]
    fn rejects_bad_modifiers() {
        let wrap = |m: &str| {
            format!(
                r##"{{"version":"2025.10","modifiers":{{"m":{m}}},"resolutionOrder":[{{"$ref":"#/modifiers/m"}}]}}"##
            )
        };
        fails(
            &wrap(r#"{"contexts": {}}"#),
            DiagnosticCode::InvalidModifier,
        );
        fails(
            &wrap(r#"{"contexts": {"a": []}}"#),
            DiagnosticCode::InvalidModifier,
        );
        fails(
            &wrap(r#"{"contexts": {"a": [], "b": []}, "default": "c"}"#),
            DiagnosticCode::InvalidModifier,
        );
        fails(
            &wrap(r#"{"default": "a"}"#),
            DiagnosticCode::InvalidResolverShape,
        );
    }

    #[test]
    fn rejects_bad_references() {
        let wrap = |src: &str| {
            format!(
                r##"{{"version":"2025.10","sets":{{"a":{{"sources":[{src}]}},"b":{{"sources":[{{"$ref":"#/sets/a"}}]}}}},
                "resolutionOrder":[{{"$ref":"#/sets/b"}}]}}"##
            )
        };
        fails(
            &wrap(r##"{"$ref": "#/sets/nope"}"##),
            DiagnosticCode::InvalidResolverReference,
        );
        fails(
            &wrap(r##"{"$ref": "#/modifiers/m"}"##),
            DiagnosticCode::InvalidResolverReference,
        );
        fails(
            &wrap(r##"{"$ref": "#/resolutionOrder/0"}"##),
            DiagnosticCode::InvalidResolverReference,
        );
        fails(
            &wrap(r##"{"$ref": "#/nothing/here"}"##),
            DiagnosticCode::InvalidResolverReference,
        );
        fails(
            &wrap(r##"{"$ref": "https://x.dev/a.json"}"##),
            DiagnosticCode::UnsupportedResolverReference,
        );
        fails(
            &wrap(r##"{"$ref": "/abs/a.json"}"##),
            DiagnosticCode::InvalidResolverReference,
        );
        fails(
            &wrap(r##"{"$ref": "a.json", "extra": 1}"##),
            DiagnosticCode::UnsupportedResolverReference,
        );
        fails(&wrap(r#"5"#), DiagnosticCode::InvalidResolverShape);
    }

    #[test]
    fn detects_cycles() {
        fails(
            r##"{"version":"2025.10","sets":{"a":{"sources":[{"$ref":"#/sets/b"}]},"b":{"sources":[{"$ref":"#/sets/a"}]}},
            "resolutionOrder":[{"$ref":"#/sets/a"}]}"##,
            DiagnosticCode::ResolverReferenceCycle,
        );
        fails(
            r##"{"version":"2025.10","sets":{"a":{"sources":[{"$ref":"#/sets/a"}]}},
            "resolutionOrder":[{"$ref":"#/sets/a"}]}"##,
            DiagnosticCode::ResolverReferenceCycle,
        );
        fails(
            r##"{"version":"2025.10","sets":{"a":{"sources":[{"$ref":"#/sets/a/sources/1"},{"$ref":"#/sets/a/sources/0"}]}},
            "resolutionOrder":[{"$ref":"#/sets/a"}]}"##,
            DiagnosticCode::ResolverReferenceCycle,
        );
        fails(
            r##"{"version":"2025.10","sets":{"a":{"sources":[{"$ref":"#"}]}},
            "resolutionOrder":[{"$ref":"#/sets/a"}]}"##,
            DiagnosticCode::ResolverReferenceCycle,
        );
    }

    #[test]
    fn follows_inline_ref_chains() {
        let doc = ok(
            r##"{"version":"2025.10","sets":{"a":{"sources":[{"$ref":"#/sets/a/sources/1"},{"x":{}}]}},
            "resolutionOrder":[{"$ref":"#/sets/a"}]}"##,
        );
        let OrderItem::Set(a) = &doc.order[0] else {
            panic!()
        };
        assert!(matches!(&a.sources[0], TokenSource::Inline { pointer, .. }
            if pointer.to_pointer_string() == "/sets/a/sources/1"));
    }
}
