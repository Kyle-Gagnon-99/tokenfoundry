//! The `lower` module turns the [`Merged`] tree into a *lowered* tree where every property has its own [`PropId`].
//!
//! Lowering is the first pass that knows what a DTCG group and token are. It checks the shape of each node and
//! reports problems, but it does not resolve anything: aliases, `$ref` targets, `$extends` and `$type` inheritance
//! are all left for later passes.
//!
//! Why a second ID space? A raw [`NodeId`] names one place in the source text. A [`PropId`] names one place in
//! the final tree. They are the same today, but `$extends` expansion will copy subtrees, and a copy needs fresh
//! IDs so the reference graph can tell `base.x` and `derived.x` apart. Every prop keeps its `origin` so
//! diagnostics can still point at what the user wrote.
//!
//! Every JSON value gets a prop, including the values inside a token's `$value`. That lets a `$ref` that points
//! deep into a value (`#/c/p/$value/components/0`) be a plain edge to a prop in the reference graph.

use indexmap::IndexMap;

use crate::{
    ast::reference::{JsonPointer, JsonReference},
    diagnostics::{
        code::DiagnosticCode,
        diagnostic::{Diagnostic, DiagnosticSink},
    },
    ir::{
        arena::Arena,
        merge::{Merged, MergedGroup, MergedNode},
        node::{DtcgValueSyntax, NodeId},
    },
};

/// Identifies one property in the lowered tree. It is an index into [`Lowered`].
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct PropId(pub u32);

#[derive(Debug, Clone)]
pub struct Prop {
    pub kind: PropKind,
    /// The raw node this prop was lowered from, used for spans.
    pub origin: NodeId,
    pub parent: Option<PropId>,
    /// Set when `$extends` expansion created this prop by copying another one. Diagnostics can use it to say
    /// "inherited from ...". The `origin` of a copy is still the raw node of the original.
    pub inherited_from: Option<PropId>,
}

#[derive(Debug, Clone)]
pub enum PropKind {
    Group(GroupData),
    Token(TokenData),
    /// Plain JSON: a token's `$value`, or the contents of `$extensions`.
    Value(ValueData),
}

/// The `$` properties shared by groups and tokens. Each one is the lowered value, so a later pass can read the
/// raw content through `origin` and so a `$ref` can point at it.
#[derive(Debug, Default, Clone, Copy)]
pub struct CommonProps {
    pub type_: Option<PropId>,
    pub description: Option<PropId>,
    pub deprecated: Option<PropId>,
    pub extensions: Option<PropId>,
}

#[derive(Debug, Default, Clone)]
pub struct GroupData {
    pub common: CommonProps,
    pub extends: Option<PropId>,
    /// Child groups and tokens by name. `$root` is a token under this key.
    pub members: IndexMap<String, PropId>,
}

#[derive(Debug, Clone)]
pub struct TokenData {
    pub common: CommonProps,
    pub body: TokenBody,
}

/// A token has exactly one of `$value` or `$ref`, and this type makes "both" and "neither" unrepresentable.
#[derive(Debug, Clone)]
pub enum TokenBody {
    Value(PropId),
    /// `prop` is the lowered `$ref` string, and `reference` is its parsed form.
    Ref {
        prop: PropId,
        reference: JsonReference,
    },
    /// The body was reported as an error. Later passes skip it.
    Invalid,
}

/// The lowered form of a plain JSON value. Scalar content is not copied, read it through [`Prop::origin`].
#[derive(Debug, Clone)]
pub enum ValueData {
    Scalar,
    Array(Vec<PropId>),
    Object(IndexMap<String, PropId>),
    Invalid,
}

#[derive(Debug)]
pub struct Lowered {
    props: Vec<Prop>,
    pub root: PropId,
}

impl Lowered {
    pub fn get(&self, id: PropId) -> &Prop {
        &self.props[id.0 as usize]
    }

    pub fn len(&self) -> usize {
        self.props.len()
    }

    pub fn is_empty(&self) -> bool {
        self.props.is_empty()
    }

    pub(crate) fn alloc(&mut self, origin: NodeId, parent: Option<PropId>, inherited_from: Option<PropId>) -> PropId {
        let id = PropId(self.props.len() as u32);
        self.props.push(Prop {
            kind: PropKind::Value(ValueData::Invalid),
            origin,
            parent,
            inherited_from,
        });
        id
    }

    pub(crate) fn get_mut(&mut self, id: PropId) -> &mut Prop {
        &mut self.props[id.0 as usize]
    }

    pub(crate) fn set_kind(&mut self, id: PropId, kind: PropKind) {
        self.props[id.0 as usize].kind = kind;
    }

    /// The props directly contained by `id` with the key each one has inside it, in order.
    /// Keys are what a JSON pointer would use: member names, `$value`, `$type`, array indexes, and so on.
    pub fn keyed_children(&self, id: PropId) -> Vec<(String, PropId)> {
        fn common(c: &CommonProps, out: &mut Vec<(String, PropId)>) {
            let named = [
                ("$type", c.type_),
                ("$description", c.description),
                ("$deprecated", c.deprecated),
                ("$extensions", c.extensions),
            ];
            out.extend(named.into_iter().filter_map(|(k, v)| Some((k.to_string(), v?))));
        }

        let mut out = Vec::new();
        match &self.get(id).kind {
            PropKind::Group(g) => {
                common(&g.common, &mut out);
                out.extend(g.extends.map(|e| ("$extends".to_string(), e)));
                out.extend(g.members.iter().map(|(k, v)| (k.clone(), *v)));
            }
            PropKind::Token(t) => {
                common(&t.common, &mut out);
                match &t.body {
                    TokenBody::Value(v) => out.push(("$value".into(), *v)),
                    TokenBody::Ref { prop, .. } => out.push(("$ref".into(), *prop)),
                    TokenBody::Invalid => {}
                }
            }
            PropKind::Value(ValueData::Array(items)) => {
                out.extend(items.iter().enumerate().map(|(i, v)| (i.to_string(), *v)));
            }
            PropKind::Value(ValueData::Object(map)) => {
                out.extend(map.iter().map(|(k, v)| (k.clone(), *v)));
            }
            PropKind::Value(_) => {}
        }
        out
    }

    /// The props directly contained by `id`, in order. This is the containment relation the graph passes need.
    pub fn children(&self, id: PropId) -> Vec<PropId> {
        self.keyed_children(id).into_iter().map(|(_, v)| v).collect()
    }

    /// The child of `id` with the given pointer key, or `None`.
    pub fn child_by_key(&self, id: PropId, key: &str) -> Option<PropId> {
        self.keyed_children(id)
            .into_iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v)
    }

    /// Resolves a JSON pointer against the lowered tree. It walks groups, tokens and plain values alike, so it
    /// can land on a group, a token, a `$`-property, or anything inside a token's value.
    /// The empty pointer is the root group.
    pub fn resolve_pointer(&self, pointer: &JsonPointer) -> Option<PropId> {
        pointer
            .segments()
            .iter()
            .try_fold(self.root, |id, key| self.child_by_key(id, key))
    }

    /// Resolves a token path (the segments of an alias such as `{color.brand.$root}`). Only group members are
    /// followed, so a `$`-property can never be reached except the `$root` token.
    pub fn member_path(&self, segments: &[String]) -> Option<PropId> {
        segments.iter().try_fold(self.root, |id, name| match &self.get(id).kind {
            PropKind::Group(g) => g.members.get(name).copied(),
            _ => None,
        })
    }

    /// The JSON pointer fragment of a prop, e.g. `#/colors/blue/$value`, for use in diagnostics.
    pub fn path_of(&self, id: PropId) -> String {
        let mut keys = Vec::new();
        let mut current = id;
        while let Some(parent) = self.get(current).parent {
            let key = self
                .keyed_children(parent)
                .into_iter()
                .find(|(_, child)| *child == current)
                .map(|(k, _)| k)
                .unwrap_or_default();
            keys.push(key);
            current = parent;
        }
        keys.reverse();
        JsonPointer::from_segments(keys).to_uri_fragment()
    }

    /// Finds a group or token by walking member names from the root.
    pub fn member(&self, path: &[&str]) -> Option<PropId> {
        let mut id = self.root;
        for name in path {
            let PropKind::Group(g) = &self.get(id).kind else {
                return None;
            };
            id = *g.members.get(*name)?;
        }
        Some(id)
    }
}

/// Lowers the merged tree. Returns `None` when nothing was layered, since there is no root to lower.
pub fn lower(arena: &Arena, merged: &Merged, diags: &mut impl DiagnosticSink) -> Option<Lowered> {
    let origin = *merged.root.origins.last()?;
    let mut lowerer = Lowerer {
        arena,
        props: Vec::new(),
        diags,
    };
    let root = lowerer.group(&merged.root, origin, None);
    Some(Lowered {
        props: lowerer.props,
        root,
    })
}

struct Lowerer<'a> {
    arena: &'a Arena,
    props: Vec<Prop>,
    diags: &'a mut dyn DiagnosticSink,
}

impl Lowerer<'_> {
    /// Reserves an ID before lowering children so a parent always has a lower ID than its descendants.
    fn reserve(&mut self, origin: NodeId, parent: Option<PropId>) -> PropId {
        let id = PropId(self.props.len() as u32);
        self.props.push(Prop {
            kind: PropKind::Value(ValueData::Invalid),
            origin,
            parent,
            inherited_from: None,
        });
        id
    }

    fn set(&mut self, id: PropId, kind: PropKind) {
        self.props[id.0 as usize].kind = kind;
    }

    fn group(&mut self, merged: &MergedGroup, origin: NodeId, parent: Option<PropId>) -> PropId {
        let id = self.reserve(origin, parent);
        let mut data = GroupData::default();

        for (key, &node) in &merged.props {
            if key == "$extends" {
                if self.shape_ok(key, node, "a string or a reference object", |v| {
                    matches!(v, DtcgValueSyntax::String(_) | DtcgValueSyntax::Object(_))
                }) {
                    data.extends = Some(self.value(node, id));
                }
            } else {
                self.common_prop(key, node, id, &mut data.common);
            }
        }

        for (name, child) in &merged.children {
            let child_id = match child {
                MergedNode::Group(sub) => {
                    let sub_origin = sub.origins.last().copied().unwrap_or(origin);
                    self.check_name(name, sub_origin);
                    self.group(sub, sub_origin, Some(id))
                }
                MergedNode::Token { node, .. } => {
                    self.check_name(name, *node);
                    self.token(*node, id)
                }
                MergedNode::Malformed(node) => {
                    self.malformed(name, *node);
                    continue;
                }
            };
            data.members.insert(name.clone(), child_id);
        }

        self.set(id, PropKind::Group(data));
        id
    }

    fn token(&mut self, node: NodeId, parent: PropId) -> PropId {
        let arena = self.arena;
        let id = self.reserve(node, Some(parent));
        let mut common = CommonProps::default();
        let (mut value, mut reference) = (None, None);

        if let DtcgValueSyntax::Object(members) = &arena.get(node).value {
            // Later duplicates overwrite earlier ones, matching the merge.
            for member in members {
                match member.key.value.as_str() {
                    "$value" => value = Some(member),
                    "$ref" => reference = Some(member),
                    key => self.common_prop(key, member.value, id, &mut common),
                }
            }
        }

        let body = match (value, reference) {
            (Some(v), Some(r)) => {
                self.diags.emit(
                    Diagnostic::new(DiagnosticCode::TokenValueAndRef)
                        .with_message("A token must have either `$value` or `$ref`, not both")
                        .primary(r.span.clone(), "`$ref` is not allowed alongside `$value`")
                        .secondary(v.span.clone(), "`$value` is defined here")
                        .help("remove one of them; `$value` is used"),
                );
                TokenBody::Value(self.value(v.value, id))
            }
            (Some(v), None) => TokenBody::Value(self.value(v.value, id)),
            (None, Some(r)) => self.reference_body(r.value, id),
            // A token-like node always has one of the two, so this only guards against misuse.
            (None, None) => TokenBody::Invalid,
        };

        self.set(id, PropKind::Token(TokenData { common, body }));
        id
    }

    fn reference_body(&mut self, node: NodeId, parent: PropId) -> TokenBody {
        let raw = self.arena.get(node);
        let text = match &raw.value {
            DtcgValueSyntax::String(s) => s,
            DtcgValueSyntax::Invalid => return TokenBody::Invalid,
            _ => {
                self.diags.emit(
                    Diagnostic::new(DiagnosticCode::InvalidPropertyShape)
                        .with_message("`$ref` must be a string")
                        .primary(raw.span.clone(), "expected a JSON pointer string such as \"#/colors/blue\""),
                );
                return TokenBody::Invalid;
            }
        };

        match JsonReference::from_str(text) {
            Ok(reference) => TokenBody::Ref {
                prop: self.value(node, parent),
                reference,
            },
            Err(err) => {
                let mut diag = Diagnostic::new(DiagnosticCode::InvalidReference)
                    .with_message(format!("`{text}` is not a valid reference"))
                    .primary(raw.span.clone(), format!("{err:?}"));
                if let Some(hint) = err.hint() {
                    diag = diag.help(hint);
                }
                self.diags.emit(diag);
                TokenBody::Invalid
            }
        }
    }

    /// Handles the `$` properties groups and tokens share, validating the shape of each.
    /// Unknown `$` properties are ignored here.
    fn common_prop(&mut self, key: &str, node: NodeId, parent: PropId, common: &mut CommonProps) {
        let (slot, expected): (&mut Option<PropId>, &str) = match key {
            "$type" => (&mut common.type_, "a string"),
            "$description" => (&mut common.description, "a string"),
            "$deprecated" => (&mut common.deprecated, "a boolean or a string"),
            "$extensions" => (&mut common.extensions, "an object"),
            _ => return,
        };

        let ok = self.shape_ok(key, node, expected, |v| match key {
            "$type" | "$description" => matches!(v, DtcgValueSyntax::String(_)),
            "$deprecated" => matches!(v, DtcgValueSyntax::Bool(_) | DtcgValueSyntax::String(_)),
            _ => matches!(v, DtcgValueSyntax::Object(_)),
        });

        if ok {
            // `slot` borrows `common`, not `self`, so lowering the value here is fine.
            *slot = Some(self.value(node, parent));
        }
    }

    /// Reports a wrong shape. `Invalid` nodes pass silently since the parser already reported them.
    fn shape_ok(
        &mut self,
        key: &str,
        node: NodeId,
        expected: &str,
        check: impl Fn(&DtcgValueSyntax) -> bool,
    ) -> bool {
        let raw = self.arena.get(node);
        if matches!(raw.value, DtcgValueSyntax::Invalid) || check(&raw.value) {
            return true;
        }
        self.diags.emit(
            Diagnostic::new(DiagnosticCode::InvalidPropertyShape)
                .with_message(format!("`{key}` has the wrong type"))
                .primary(raw.span.clone(), format!("expected {expected}")),
        );
        false
    }

    /// Names must not contain `{`, `}` or `.`, because those characters are alias syntax. `$` is reserved.
    /// `$root` never reaches here as a group or a token name by accident, it is the one allowed `$` name.
    fn check_name(&mut self, name: &str, node: NodeId) {
        if name == "$root" {
            return;
        }
        let bad = name.starts_with('$') || name.contains(['{', '}', '.']);
        if bad {
            self.diags.emit(
                Diagnostic::new(DiagnosticCode::InvalidName)
                    .with_message(format!("`{name}` is not a valid group or token name"))
                    .primary(
                        self.arena.get(node).span.clone(),
                        "names cannot start with `$` or contain `{`, `}` or `.`",
                    ),
            );
        }
    }

    fn malformed(&mut self, name: &str, node: NodeId) {
        let raw = self.arena.get(node);
        if matches!(raw.value, DtcgValueSyntax::Invalid) {
            return;
        }
        self.diags.emit(
            Diagnostic::new(DiagnosticCode::InvalidNodeShape)
                .with_message(format!("`{name}` must be a group or a token"))
                .primary(raw.span.clone(), "expected an object")
                .help("a token is an object with `$value` or `$ref`, and a group is any other object"),
        );
    }

    /// Lowers plain JSON recursively, giving every value its own prop.
    fn value(&mut self, node: NodeId, parent: PropId) -> PropId {
        let arena = self.arena;
        let id = self.reserve(node, Some(parent));

        let data = match &arena.get(node).value {
            DtcgValueSyntax::Array(items) => {
                ValueData::Array(items.iter().map(|&item| self.value(item, id)).collect())
            }
            DtcgValueSyntax::Object(members) => {
                let mut map = IndexMap::new();
                for (i, member) in members.iter().enumerate() {
                    // Only the last duplicate key survives, so earlier ones get no prop.
                    if members[i + 1..].iter().any(|m| m.key.value == member.key.value) {
                        continue;
                    }
                    map.insert(member.key.value.clone(), self.value(member.value, id));
                }
                ValueData::Object(map)
            }
            DtcgValueSyntax::Invalid => ValueData::Invalid,
            _ => ValueData::Scalar,
        };

        self.set(id, PropKind::Value(data));
        id
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        diagnostics::diagnostic::DiagnosticStore,
        ir::{arena::build, merge::LayerOrigin},
        source::SourceIdGenerator,
    };

    struct Fixture {
        arena: Arena,
        ids: SourceIdGenerator,
        merged: Merged,
        diags: DiagnosticStore,
    }

    impl Fixture {
        fn new() -> Self {
            Self {
                arena: Arena::default(),
                ids: SourceIdGenerator::default(),
                merged: Merged::new(),
                diags: DiagnosticStore::new(),
            }
        }

        fn layer(&mut self, json: &str) -> &mut Self {
            let (tree, errors) = json_spanned::parse_json(json);
            assert!(errors.is_empty(), "syntax errors: {errors:?}");
            let source = self.ids.generate();
            let root = build(&mut self.arena, source, &tree.unwrap(), None);
            self.merged
                .layer(&self.arena, root, LayerOrigin::File(source), &mut self.diags);
            self
        }

        fn lower(&mut self) -> Lowered {
            lower(&self.arena, &self.merged, &mut self.diags).expect("something was layered")
        }

        fn codes(&self) -> Vec<DiagnosticCode> {
            self.diags.diagnostics().iter().map(|d| d.code).collect()
        }
    }

    /// Lowers a single document and returns the fixture and result.
    fn one(json: &str) -> (Fixture, Lowered) {
        let mut f = Fixture::new();
        f.layer(json);
        let l = f.lower();
        (f, l)
    }

    fn token<'a>(l: &'a Lowered, path: &[&str]) -> &'a TokenData {
        match &l.get(l.member(path).unwrap_or_else(|| panic!("no member {path:?}"))).kind {
            PropKind::Token(t) => t,
            other => panic!("expected token at {path:?}, got {other:?}"),
        }
    }

    fn group<'a>(l: &'a Lowered, path: &[&str]) -> &'a GroupData {
        match &l.get(l.member(path).unwrap_or_else(|| panic!("no member {path:?}"))).kind {
            PropKind::Group(g) => g,
            other => panic!("expected group at {path:?}, got {other:?}"),
        }
    }

    fn string_of(f: &Fixture, l: &Lowered, id: PropId) -> String {
        match &f.arena.get(l.get(id).origin).value {
            DtcgValueSyntax::String(s) => s.clone(),
            other => panic!("expected string, got {other:?}"),
        }
    }

    #[test]
    fn lowers_groups_and_tokens() {
        let (f, l) = one(r#"{"color": {"red": {"$value": 1}, "dark": {"x": {"$value": 2}}}}"#);

        assert!(matches!(&l.get(l.root).kind, PropKind::Group(_)));
        assert_eq!(group(&l, &[]).members.keys().collect::<Vec<_>>(), ["color"]);
        assert_eq!(group(&l, &["color"]).members.keys().collect::<Vec<_>>(), ["red", "dark"]);
        assert!(matches!(token(&l, &["color", "red"]).body, TokenBody::Value(_)));
        group(&l, &["color", "dark"]);
        token(&l, &["color", "dark", "x"]);
        assert!(f.diags.diagnostics().is_empty());
    }

    #[test]
    fn parents_always_have_lower_ids_and_links_are_consistent() {
        let (_, l) = one(r#"{"g": {"t": {"$type": "color", "$value": {"c": [1, {"d": 2}]}}}}"#);

        for i in 0..l.len() as u32 {
            let id = PropId(i);
            for child in l.children(id) {
                assert_eq!(l.get(child).parent, Some(id));
                assert!(child > id);
            }
        }
        assert_eq!(l.get(l.root).parent, None);
    }

    #[test]
    fn every_json_value_in_a_token_value_gets_a_prop() {
        // root, g, t, $type, the $value object, the array, 1, {d}, 2 (keys do not get props)
        let (_, l) = one(r#"{"g": {"t": {"$type": "color", "$value": {"c": [1, {"d": 2}]}}}}"#);
        assert_eq!(l.len(), 9);
    }

    #[test]
    fn token_value_is_lowered_as_plain_json() {
        let (_, l) = one(r#"{"t": {"$value": {"a": [1, 2]}}}"#);
        let TokenBody::Value(v) = token(&l, &["t"]).body else { panic!() };

        let PropKind::Value(ValueData::Object(map)) = &l.get(v).kind else { panic!() };
        let PropKind::Value(ValueData::Array(items)) = &l.get(map["a"]).kind else { panic!() };
        assert_eq!(items.len(), 2);
        assert!(matches!(&l.get(items[0]).kind, PropKind::Value(ValueData::Scalar)));
    }

    #[test]
    fn duplicate_keys_inside_a_value_keep_the_last() {
        let (f, l) = one(r#"{"t": {"$value": {"k": 1, "k": 2}}}"#);
        let TokenBody::Value(v) = token(&l, &["t"]).body else { panic!() };
        let PropKind::Value(ValueData::Object(map)) = &l.get(v).kind else { panic!() };

        assert_eq!(map.len(), 1);
        let origin = l.get(map["k"]).origin;
        assert!(matches!(&f.arena.get(origin).value, DtcgValueSyntax::Number(n) if n.as_u64() == Some(2)));
        // root, t, $value, k
        assert_eq!(l.len(), 4);
    }

    #[test]
    fn ref_token_is_parsed() {
        let (f, l) = one(r##"{"a": {"$ref": "#/b/$value"}, "b": {"$value": 1}}"##);

        let TokenBody::Ref { prop, reference } = &token(&l, &["a"]).body else { panic!() };
        assert!(reference.document().is_none());
        assert_eq!(reference.fragment().segments(), ["b", "$value"]);
        assert_eq!(string_of(&f, &l, *prop), "#/b/$value");
        assert!(f.diags.diagnostics().is_empty());
    }

    #[test]
    fn ref_to_another_file_is_parsed() {
        let (_, l) = one(r##"{"a": {"$ref": "colors.json#/blue"}}"##);
        let TokenBody::Ref { reference, .. } = &token(&l, &["a"]).body else { panic!() };
        assert_eq!(reference.document().unwrap().path(), Some("colors.json"));
    }

    #[test]
    fn value_and_ref_together_is_an_error_and_value_wins() {
        let (f, l) = one(r##"{"a": {"$value": 1, "$ref": "#/b"}}"##);

        assert_eq!(f.codes(), [DiagnosticCode::TokenValueAndRef]);
        assert_eq!(f.diags.diagnostics()[0].labels.len(), 2);
        assert!(matches!(token(&l, &["a"]).body, TokenBody::Value(_)));
    }

    #[test]
    fn non_string_ref_is_a_shape_error() {
        let (f, l) = one(r#"{"a": {"$ref": 5}}"#);
        assert_eq!(f.codes(), [DiagnosticCode::InvalidPropertyShape]);
        assert!(matches!(token(&l, &["a"]).body, TokenBody::Invalid));
    }

    #[test]
    fn bad_ref_reports_the_hint_from_the_reference_parser() {
        let (f, l) = one(r#"{"a": {"$ref": "C:\\tokens\\colors.json#/x"}}"#);

        assert_eq!(f.codes(), [DiagnosticCode::InvalidReference]);
        assert!(f.diags.diagnostics()[0].help.as_deref().unwrap().contains("relative"));
        assert!(matches!(token(&l, &["a"]).body, TokenBody::Invalid));

        let (f, _) = one(r#"{"a": {"$ref": "https://example.com/a.json#/x"}}"#);
        assert_eq!(f.codes(), [DiagnosticCode::InvalidReference]);
    }

    #[test]
    fn common_props_are_lowered_on_tokens_and_groups() {
        let (f, l) = one(
            r#"{"g": {"$type": "color", "$description": "d", "$deprecated": "use h",
                       "$extensions": {"x": 1},
                       "t": {"$type": "dimension", "$deprecated": true, "$value": 1}}}"#,
        );

        let g = group(&l, &["g"]);
        assert_eq!(string_of(&f, &l, g.common.type_.unwrap()), "color");
        assert_eq!(string_of(&f, &l, g.common.description.unwrap()), "d");
        assert!(g.common.deprecated.is_some() && g.common.extensions.is_some());

        let t = token(&l, &["g", "t"]);
        assert_eq!(string_of(&f, &l, t.common.type_.unwrap()), "dimension");
        assert!(t.common.deprecated.is_some());
        assert!(t.common.description.is_none());
        assert!(f.diags.diagnostics().is_empty());
    }

    #[test]
    fn wrong_common_prop_shapes_are_reported_and_dropped() {
        let (f, l) = one(
            r#"{"g": {"$type": 5, "$description": [], "$deprecated": 1, "$extensions": [],
                       "t": {"$type": {}, "$value": 1}}}"#,
        );

        assert_eq!(f.codes(), [DiagnosticCode::InvalidPropertyShape; 5]);
        let g = group(&l, &["g"]);
        assert!(g.common.type_.is_none() && g.common.description.is_none());
        assert!(g.common.deprecated.is_none() && g.common.extensions.is_none());
        assert!(token(&l, &["g", "t"]).common.type_.is_none());
    }

    #[test]
    fn extends_accepts_a_string_or_an_object() {
        let (f, l) = one(r##"{"a": {"$extends": "{b}"}, "c": {"$extends": {"$ref": "#/b"}}, "d": {"$extends": 1}}"##);

        assert!(group(&l, &["a"]).extends.is_some());
        assert!(group(&l, &["c"]).extends.is_some());
        assert!(group(&l, &["d"]).extends.is_none());
        assert_eq!(f.codes(), [DiagnosticCode::InvalidPropertyShape]);
    }

    #[test]
    fn root_is_a_token_member_of_the_group() {
        let (f, l) = one(r#"{"g": {"$root": {"$value": 1}, "x": {"$value": 2}}}"#);

        token(&l, &["g", "$root"]);
        assert_eq!(group(&l, &["g"]).members.keys().collect::<Vec<_>>(), ["$root", "x"]);
        assert!(f.diags.diagnostics().is_empty());
    }

    #[test]
    fn invalid_names_are_reported_but_still_lowered() {
        let (f, l) = one(r#"{"a.b": {"$value": 1}, "{x}": {"$value": 1}, "c}": {"y": {"$value": 1}}, "ok": {"$value": 1}}"#);

        assert_eq!(f.codes(), [DiagnosticCode::InvalidName; 3]);
        assert_eq!(group(&l, &[]).members.len(), 4);
    }

    #[test]
    fn malformed_children_are_reported_and_left_out() {
        let (f, l) = one(r#"{"n": 5, "arr": [1], "ok": {"$value": 1}}"#);

        assert_eq!(f.codes(), [DiagnosticCode::InvalidNodeShape; 2]);
        assert_eq!(group(&l, &[]).members.keys().collect::<Vec<_>>(), ["ok"]);
    }

    #[test]
    fn lowering_reflects_the_merge_winners() {
        let mut f = Fixture::new();
        f.layer(r#"{"g": {"$type": "color", "t": {"$type": "color", "$value": 1}, "u": {"$value": 2}}}"#);
        f.layer(r#"{"g": {"$description": "later", "t": {"$value": 3}}}"#);
        let l = f.lower();

        // `t` was replaced whole, so its `$type` is gone.
        assert!(token(&l, &["g", "t"]).common.type_.is_none());
        // The group keeps `$type` from layer one and gets `$description` from layer two.
        let g = group(&l, &["g"]);
        assert!(g.common.type_.is_some() && g.common.description.is_some());
        assert_eq!(g.members.len(), 2);
        // The group's origin is the latest contributing object.
        assert_eq!(l.get(l.member(&["g"]).unwrap()).origin, *f.merged.root.get(&crate::ast::reference::JsonPointer::from_segments(["g"])).map(|n| match n {
            MergedNode::Group(g) => g.origins.last().unwrap(),
            _ => panic!(),
        }).unwrap());
    }

    #[test]
    fn nothing_layered_lowers_to_none() {
        let mut f = Fixture::new();
        assert!(lower(&f.arena, &f.merged, &mut f.diags).is_none());
        f.layer("[1]");
        assert!(lower(&f.arena, &f.merged, &mut f.diags).is_none());
    }

    #[test]
    fn origins_point_back_to_the_raw_nodes() {
        let (f, l) = one(r#"{"t": {"$value": 1}}"#);
        let id = l.member(&["t"]).unwrap();
        let raw = f.arena.get(l.get(id).origin);
        assert!(matches!(raw.value, DtcgValueSyntax::Object(_)));
    }
}
