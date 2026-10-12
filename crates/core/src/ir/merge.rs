//! The `merge` module layers the raw documents held in the [`Arena`] on top of each other into one [`Merged`] tree.
//!
//! The merge never copies or edits raw nodes. It only decides, for every position in the final token tree, which raw `NodeId`
//! wins. The rules follow the DTCG spec: later layers win, groups merge key by key, and a token is replaced as a whole
//! (never property by property).
//!
//! The merge is purely syntactic. It only needs to know whether an object is token-like (see [`RawNode::is_token_like`]).
//! Shape validation, alias resolution, and `$extends` all happen later, against the merged tree.

use indexmap::IndexMap;

use crate::{
    ast::reference::JsonPointer,
    diagnostics::{
        code::DiagnosticCode,
        diagnostic::{Diagnostic, DiagnosticSink},
    },
    ir::{
        arena::Arena,
        node::{DtcgValueSyntax, NodeId},
    },
    source::SourceId,
};

/// Describes where a layer came from. This is separate from where its text lives: inline resolver tokens
/// are bytes inside the resolver file, so their spans use the resolver's `SourceId`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LayerOrigin {
    /// A whole token file.
    File(SourceId),
    /// Tokens written inline inside a resolver document, e.g. `#/sets/base/sources/1`.
    Inline {
        resolver: SourceId,
        pointer: JsonPointer,
    },
}

/// A group in the merged tree.
#[derive(Debug, Default)]
pub struct MergedGroup {
    /// `$`-prefixed members of the group (`$type`, `$description`, `$extends`, ...). The later layer wins per key.
    /// `$root` is not here, it is a token slot and lives in `children`.
    pub props: IndexMap<String, NodeId>,
    /// Child tokens and groups by name, in first-seen order. Overriding a key keeps its original position.
    pub children: IndexMap<String, MergedNode>,
    /// Every raw object that contributed to this group, in layering order.
    pub origins: Vec<NodeId>,
}

impl MergedGroup {
    /// Looks up a child by walking a path of names from this group.
    pub fn get(&self, path: &[&str]) -> Option<&MergedNode> {
        let (first, rest) = path.split_first()?;
        let node = self.children.get(*first)?;
        if rest.is_empty() {
            return Some(node);
        }
        match node {
            MergedNode::Group(group) => group.get(rest),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub enum MergedNode {
    Group(MergedGroup),
    /// The winning raw token object, taken whole. `layer` indexes [`Merged::layers`].
    Token {
        node: NodeId,
        layer: usize,
    },
    /// Something that is neither a group nor a token (a number, an array, a recovered `Invalid`, ...).
    /// It is kept so the lowering pass can report the shape error with a span, instead of the merge guessing.
    Malformed(NodeId),
}

/// Records that a token was replaced by a later one, so diagnostics can say where the losing value came from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Shadow {
    /// The path of names from the root to the replaced token.
    pub path: Vec<String>,
    pub loser: NodeId,
    pub loser_layer: usize,
    pub winner: NodeId,
    pub winner_layer: usize,
}

#[derive(Debug, Default)]
pub struct Merged {
    pub root: MergedGroup,
    /// The layers merged so far, in order. Index into this with a `layer` field.
    pub layers: Vec<LayerOrigin>,
    pub shadows: Vec<Shadow>,
}

/// Everything the recursive walk needs besides the group it is currently filling.
struct Ctx<'a> {
    arena: &'a Arena,
    layer: usize,
    shadows: &'a mut Vec<Shadow>,
    diags: &'a mut dyn DiagnosticSink,
}

impl Merged {
    pub fn new() -> Self {
        Self::default()
    }

    /// Layers the object rooted at `root` over everything merged so far. The caller decides the order.
    /// `root` is usually a document root, but it can be any object node, such as an inline resolver source.
    pub fn layer(
        &mut self,
        arena: &Arena,
        root: NodeId,
        origin: LayerOrigin,
        diags: &mut impl DiagnosticSink,
    ) {
        let node = arena.get(root);

        match &node.value {
            // Already reported as a syntax error by the parser, nothing to merge.
            DtcgValueSyntax::Invalid => return,
            DtcgValueSyntax::Object(_) if !node.is_token_like() => {}
            _ => {
                diags.emit(
                    Diagnostic::new(DiagnosticCode::InvalidMergeRoot)
                        .with_message("A token file must have a group as its root")
                        .primary(
                            node.span.clone(),
                            "expected an object without `$value` or `$ref`",
                        ),
                );
                return;
            }
        }

        self.layers.push(origin);
        let mut ctx = Ctx {
            arena,
            layer: self.layers.len() - 1,
            shadows: &mut self.shadows,
            diags,
        };
        layer_group(&mut self.root, &mut ctx, root, &mut Vec::new());
    }
}

fn layer_group(group: &mut MergedGroup, ctx: &mut Ctx, obj: NodeId, path: &mut Vec<String>) {
    group.origins.push(obj);

    let DtcgValueSyntax::Object(members) = &ctx.arena.get(obj).value else {
        return;
    };

    // Members are visited in source order, so with a duplicate key inside one object the last one wins.
    for member in members {
        let key = &member.key.value;
        let child_id = member.value;
        let child = ctx.arena.get(child_id);

        if key.starts_with('$') && key != "$root" {
            group.props.insert(key.clone(), child_id);
            continue;
        }

        path.push(key.clone());

        if child.is_token_like() {
            if let Some(old) = group.children.get(key) {
                replace_report(ctx, path, old, child_id);
            }
            group.children.insert(
                key.clone(),
                MergedNode::Token {
                    node: child_id,
                    layer: ctx.layer,
                },
            );
        } else if matches!(child.value, DtcgValueSyntax::Object(_)) {
            if !matches!(group.children.get(key), Some(MergedNode::Group(_))) {
                if let Some(old) = group.children.get(key) {
                    replace_report(ctx, path, old, child_id);
                }
                group
                    .children
                    .insert(key.clone(), MergedNode::Group(MergedGroup::default()));
            }
            if let Some(MergedNode::Group(sub)) = group.children.get_mut(key) {
                layer_group(sub, ctx, child_id, path);
            }
        } else {
            group
                .children
                .insert(key.clone(), MergedNode::Malformed(child_id));
        }

        path.pop();
    }
}

/// Called when `winner` (a raw node from the current layer) lands on a position that already holds `old`.
/// Token over token is normal overriding, so it is only recorded. A group and a token colliding is warned about.
fn replace_report(ctx: &mut Ctx, path: &[String], old: &MergedNode, winner: NodeId) {
    let winner_node = ctx.arena.get(winner);

    match old {
        MergedNode::Token { node, layer } if winner_node.is_token_like() => {
            ctx.shadows.push(Shadow {
                path: path.to_vec(),
                loser: *node,
                loser_layer: *layer,
                winner,
                winner_layer: ctx.layer,
            });
        }
        MergedNode::Token { node, .. } => conflict(ctx, path, "a group", "a token", winner, *node),
        MergedNode::Group(group) if winner_node.is_token_like() => {
            if let Some(loser) = group.origins.last() {
                conflict(ctx, path, "a token", "a group", winner, *loser);
            }
        }
        // A group over a group never reaches here, and a malformed node has already been reported.
        _ => {}
    }
}

fn conflict(
    ctx: &mut Ctx,
    path: &[String],
    winner_kind: &str,
    loser_kind: &str,
    winner: NodeId,
    loser: NodeId,
) {
    ctx.diags.emit(
        Diagnostic::new(DiagnosticCode::GroupTokenConflict)
            .with_message(format!(
                "`{}` is {loser_kind} in an earlier layer but {winner_kind} in a later one",
                path.join(".")
            ))
            .primary(
                ctx.arena.get(winner).span.clone(),
                format!("{winner_kind} replaces the earlier definition"),
            )
            .secondary(
                ctx.arena.get(loser).span.clone(),
                format!("earlier definition as {loser_kind}"),
            ),
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        diagnostics::diagnostic::DiagnosticStore, ir::arena::build, source::SourceIdGenerator,
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

        /// Parses `json` into the shared arena as a new source and layers it on.
        fn layer(&mut self, json: &str) -> NodeId {
            let (tree, errors) = json_spanned::parse_json(json);
            assert!(errors.is_empty(), "unexpected syntax errors: {errors:?}");
            let source = self.ids.generate();
            let root = build(&mut self.arena, source, &tree.unwrap(), None);
            self.merged.layer(
                &self.arena,
                root,
                LayerOrigin::File(source),
                &mut self.diags,
            );
            root
        }

        fn node(&self, path: &[&str]) -> &MergedNode {
            self.merged
                .root
                .get(path)
                .unwrap_or_else(|| panic!("no node at {path:?}"))
        }

        /// The winning token's raw `$value` as a number, for asserting which layer won.
        fn token_number(&self, path: &[&str]) -> u64 {
            let MergedNode::Token { node, .. } = self.node(path) else {
                panic!("expected a token at {path:?}");
            };
            let DtcgValueSyntax::Object(members) = &self.arena.get(*node).value else {
                panic!("token must be an object");
            };
            let value = members
                .iter()
                .find(|m| m.key.value == "$value")
                .unwrap()
                .value;
            match &self.arena.get(value).value {
                DtcgValueSyntax::Number(n) => n.as_u64().unwrap(),
                other => panic!("expected number, got {other:?}"),
            }
        }

        fn codes(&self) -> Vec<DiagnosticCode> {
            self.diags.diagnostics().iter().map(|d| d.code).collect()
        }
    }

    fn keys(group: &MergedGroup) -> Vec<&str> {
        group.children.keys().map(String::as_str).collect()
    }

    #[test]
    fn single_layer_builds_groups_and_tokens() {
        let mut f = Fixture::new();
        f.layer(r#"{"color": {"red": {"$value": 1}, "blue": {"$value": 2}}}"#);

        assert!(matches!(f.node(&["color"]), MergedNode::Group(_)));
        assert_eq!(f.token_number(&["color", "red"]), 1);
        assert_eq!(f.token_number(&["color", "blue"]), 2);
        assert!(f.diags.diagnostics().is_empty());
    }

    #[test]
    fn groups_merge_key_by_key() {
        let mut f = Fixture::new();
        f.layer(r#"{"color": {"red": {"$value": 1}}}"#);
        f.layer(r#"{"color": {"blue": {"$value": 2}}, "size": {"s": {"$value": 3}}}"#);

        assert_eq!(keys(&f.merged.root), ["color", "size"]);
        assert_eq!(f.token_number(&["color", "red"]), 1);
        assert_eq!(f.token_number(&["color", "blue"]), 2);
        assert_eq!(f.token_number(&["size", "s"]), 3);
        assert!(f.diags.diagnostics().is_empty());
    }

    #[test]
    fn later_token_replaces_earlier_token_whole() {
        let mut f = Fixture::new();
        f.layer(r#"{"c": {"$type": "color", "$description": "old", "$value": 1}}"#);
        f.layer(r#"{"c": {"$value": 2}}"#);

        assert_eq!(f.token_number(&["c"]), 2);
        // The old `$type` and `$description` must not leak into the winner.
        let MergedNode::Token { node, .. } = f.node(&["c"]) else {
            panic!()
        };
        let DtcgValueSyntax::Object(members) = &f.arena.get(*node).value else {
            panic!()
        };
        assert_eq!(members.len(), 1);
    }

    #[test]
    fn token_replacement_records_a_shadow_with_layers() {
        let mut f = Fixture::new();
        let first = f.layer(r#"{"a": {"b": {"$value": 1}}}"#);
        let second = f.layer(r#"{"a": {"b": {"$value": 2}}}"#);

        assert_eq!(f.merged.shadows.len(), 1);
        let shadow = &f.merged.shadows[0];
        assert_eq!(shadow.path, ["a", "b"]);
        assert_eq!((shadow.loser_layer, shadow.winner_layer), (0, 1));
        assert_ne!(shadow.loser, shadow.winner);
        assert_eq!(
            f.arena.get(shadow.loser).span.source_id,
            f.arena.get(first).span.source_id
        );
        assert_eq!(
            f.arena.get(shadow.winner).span.source_id,
            f.arena.get(second).span.source_id
        );
        // Plain overriding is normal and not diagnosed.
        assert!(f.diags.diagnostics().is_empty());
    }

    #[test]
    fn ref_token_replaces_value_token_and_back() {
        let mut f = Fixture::new();
        f.layer(r#"{"a": {"$value": 1}}"#);
        f.layer(r##"{"a": {"$ref": "#/b/$value"}}"##);
        assert!(matches!(f.node(&["a"]), MergedNode::Token { layer: 1, .. }));

        f.layer(r#"{"a": {"$value": 3}}"#);
        assert_eq!(f.token_number(&["a"]), 3);
        assert_eq!(f.merged.shadows.len(), 2);
        assert!(f.diags.diagnostics().is_empty());
    }

    #[test]
    fn color_example_later_file_replaces_the_whole_color() {
        let mut f = Fixture::new();
        f.layer(r#"{"p": {"$value": {"components": [1, 2, 3]}}}"#);
        f.layer(r##"{"p": {"$value": {"components": [{"$ref": "#/x"}]}}}"##);

        let MergedNode::Token { node, layer } = f.node(&["p"]) else {
            panic!()
        };
        assert_eq!(*layer, 1);
        // Only the later document's value exists, nothing was combined with the first one.
        let DtcgValueSyntax::Object(members) = &f.arena.get(*node).value else {
            panic!()
        };
        let value = members[0].value;
        let DtcgValueSyntax::Object(color) = &f.arena.get(value).value else {
            panic!()
        };
        let DtcgValueSyntax::Array(components) = &f.arena.get(color[0].value).value else {
            panic!()
        };
        assert_eq!(components.len(), 1);
    }

    #[test]
    fn token_over_group_warns_and_replaces() {
        let mut f = Fixture::new();
        f.layer(r#"{"a": {"b": {"$value": 1}, "c": {"$value": 2}}}"#);
        f.layer(r#"{"a": {"$value": 9}}"#);

        assert_eq!(f.token_number(&["a"]), 9);
        assert!(f.merged.root.get(&["a", "b"]).is_none());
        assert_eq!(f.codes(), [DiagnosticCode::GroupTokenConflict]);
        let diag = &f.diags.diagnostics()[0];
        assert_eq!(diag.labels.len(), 2);
        assert!(diag.message.contains("`a`"));
    }

    #[test]
    fn group_over_token_warns_and_replaces() {
        let mut f = Fixture::new();
        f.layer(r#"{"a": {"$value": 1}}"#);
        f.layer(r#"{"a": {"b": {"$value": 2}}}"#);

        assert!(matches!(f.node(&["a"]), MergedNode::Group(_)));
        assert_eq!(f.token_number(&["a", "b"]), 2);
        assert_eq!(f.codes(), [DiagnosticCode::GroupTokenConflict]);
        assert!(f.merged.shadows.is_empty());
    }

    #[test]
    fn group_props_override_per_key() {
        let mut f = Fixture::new();
        f.layer(r#"{"g": {"$type": "color", "$description": "first", "t": {"$value": 1}}}"#);
        f.layer(r#"{"g": {"$description": "second", "$deprecated": true}}"#);

        let MergedNode::Group(g) = f.node(&["g"]) else {
            panic!()
        };
        assert_eq!(
            g.props.keys().map(String::as_str).collect::<Vec<_>>(),
            ["$type", "$description", "$deprecated"]
        );
        // `$type` survives from the first layer, `$description` comes from the second.
        let desc = g.props["$description"];
        assert!(matches!(&f.arena.get(desc).value, DtcgValueSyntax::String(s) if s == "second"));
        assert_eq!(f.token_number(&["g", "t"]), 1);
        assert_eq!(g.origins.len(), 2);
    }

    #[test]
    fn root_slot_is_a_token_replaced_whole() {
        let mut f = Fixture::new();
        f.layer(r#"{"g": {"$root": {"$value": 1}, "x": {"$value": 5}}}"#);
        f.layer(r#"{"g": {"$root": {"$value": 2}}}"#);

        assert_eq!(f.token_number(&["g", "$root"]), 2);
        assert_eq!(f.token_number(&["g", "x"]), 5);
        let MergedNode::Group(g) = f.node(&["g"]) else {
            panic!()
        };
        assert!(!g.props.contains_key("$root"));
    }

    #[test]
    fn overriding_keeps_original_position() {
        let mut f = Fixture::new();
        f.layer(r#"{"a": {"$value": 1}, "b": {"$value": 2}, "c": {"$value": 3}}"#);
        f.layer(r#"{"c": {"$value": 30}, "a": {"$value": 10}, "d": {"$value": 4}}"#);

        assert_eq!(keys(&f.merged.root), ["a", "b", "c", "d"]);
        assert_eq!(f.token_number(&["a"]), 10);
    }

    #[test]
    fn duplicate_keys_in_one_object_last_wins() {
        let mut f = Fixture::new();
        f.layer(r#"{"a": {"$value": 1}, "a": {"$value": 2}}"#);

        assert_eq!(f.token_number(&["a"]), 2);
        assert_eq!(f.merged.shadows.len(), 1);
    }

    #[test]
    fn malformed_members_are_kept_for_lowering() {
        let mut f = Fixture::new();
        f.layer(r#"{"n": 5, "arr": [1], "ok": {"$value": 1}}"#);

        assert!(matches!(f.node(&["n"]), MergedNode::Malformed(_)));
        assert!(matches!(f.node(&["arr"]), MergedNode::Malformed(_)));
        assert!(matches!(f.node(&["ok"]), MergedNode::Token { .. }));
        assert!(f.diags.diagnostics().is_empty());
    }

    /// Builds `{"bad": <Invalid>, "ok": {"$value": 1}}` by hand, as the parser's recovery would.
    fn recovered_object() -> json_spanned::parser::Spanned<json_spanned::parser::JsonValue> {
        use json_spanned::parser::{JsonMember, JsonValue, Spanned};
        fn sp<T>(value: T) -> Spanned<T> {
            Spanned {
                value,
                span: (0..1).into(),
            }
        }
        let member = |k: &str, v| JsonMember {
            key: sp(k.to_string()),
            value: v,
        };
        let ok = sp(JsonValue::Object(vec![member(
            "$value",
            sp(JsonValue::Null),
        )]));
        sp(JsonValue::Object(vec![
            member("bad", sp(JsonValue::Invalid)),
            member("ok", ok),
        ]))
    }

    #[test]
    fn invalid_members_are_kept_without_new_diagnostics() {
        let mut f = Fixture::new();
        let source = f.ids.generate();
        let root = build(&mut f.arena, source, &recovered_object(), None);
        f.merged
            .layer(&f.arena, root, LayerOrigin::File(source), &mut f.diags);

        assert!(f.diags.diagnostics().is_empty());
        assert!(matches!(f.node(&["bad"]), MergedNode::Malformed(_)));
        assert!(matches!(f.node(&["ok"]), MergedNode::Token { .. }));
    }

    #[test]
    fn invalid_root_is_skipped_silently() {
        let mut f = Fixture::new();
        let source = f.ids.generate();
        let invalid = json_spanned::parser::Spanned {
            value: json_spanned::parser::JsonValue::Invalid,
            span: (0..1).into(),
        };
        let root = build(&mut f.arena, source, &invalid, None);
        f.merged
            .layer(&f.arena, root, LayerOrigin::File(source), &mut f.diags);

        assert!(f.diags.diagnostics().is_empty());
        assert!(f.merged.layers.is_empty());
    }

    #[test]
    fn non_group_roots_are_rejected() {
        for json in [
            "[1]",
            "5",
            r#""x""#,
            "null",
            r#"{"$value": 1}"#,
            r##"{"$ref": "#/a"}"##,
        ] {
            let mut f = Fixture::new();
            f.layer(json);
            assert_eq!(
                f.codes(),
                [DiagnosticCode::InvalidMergeRoot],
                "input: {json}"
            );
            assert!(f.merged.layers.is_empty(), "input: {json}");
            assert!(f.merged.root.children.is_empty());
        }
    }

    #[test]
    fn rejected_root_does_not_disturb_earlier_layers() {
        let mut f = Fixture::new();
        f.layer(r#"{"a": {"$value": 1}}"#);
        f.layer("[1]");
        f.layer(r#"{"b": {"$value": 2}}"#);

        assert_eq!(keys(&f.merged.root), ["a", "b"]);
        assert_eq!(f.merged.layers.len(), 2);
        // Layer indexes count accepted layers only.
        assert!(matches!(f.node(&["b"]), MergedNode::Token { layer: 1, .. }));
    }

    #[test]
    fn inline_layer_can_be_any_object_node_in_the_arena() {
        // A resolver document whose inline source is a nested object.
        let mut f = Fixture::new();
        f.layer(r#"{"base": {"x": {"$value": 1}}}"#);

        let (tree, errors) = json_spanned::parse_json(
            r#"{"sets": {"s": {"sources": [{"base": {"x": {"$value": 7}}}]}}}"#,
        );
        assert!(errors.is_empty());
        let resolver = f.ids.generate();
        let root = build(&mut f.arena, resolver, &tree.unwrap(), None);

        let sources = find(
            &f.arena,
            find(&f.arena, find(&f.arena, root, "sets"), "s"),
            "sources",
        );
        let DtcgValueSyntax::Array(items) = &f.arena.get(sources).value else {
            panic!()
        };
        let inline = items[0];

        let origin = LayerOrigin::Inline {
            resolver,
            pointer: JsonPointer::from_segments(["sets", "s", "sources", "0"]),
        };
        f.merged
            .layer(&f.arena, inline, origin.clone(), &mut f.diags);

        assert_eq!(f.token_number(&["base", "x"]), 7);
        assert_eq!(f.merged.layers[1], origin);
        // The inline node's span still points into the resolver source.
        assert_eq!(f.arena.get(inline).span.source_id, resolver);
    }

    fn find(arena: &Arena, obj: NodeId, key: &str) -> NodeId {
        let DtcgValueSyntax::Object(members) = &arena.get(obj).value else {
            panic!()
        };
        members.iter().find(|m| m.key.value == key).unwrap().value
    }

    #[test]
    fn get_walks_paths_and_stops_at_tokens() {
        let mut f = Fixture::new();
        f.layer(r#"{"a": {"b": {"c": {"$value": 1}}}}"#);

        assert!(f.merged.root.get(&["a", "b", "c"]).is_some());
        assert!(f.merged.root.get(&["a", "x"]).is_none());
        assert!(f.merged.root.get(&["a", "b", "c", "d"]).is_none());
        assert!(f.merged.root.get(&[]).is_none());
    }
}
