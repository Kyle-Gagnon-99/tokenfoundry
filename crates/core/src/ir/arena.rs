//! The `arena` module in `core` provides the definitions for the arena that stores IR nodes.
//! The arena is responsible for managing the memory and lifetimes of the nodes within the IR.
//! It allows efficient allocation and retrieval of nodes by their `NodeId`.

use json_spanned::parser::JsonValue;

use crate::{
    ir::node::{DtcgMemberSyntax, DtcgValueSyntax, NodeId, RawNode},
    source::{SourceId, SourceSpan, Spanned},
};

#[derive(Debug, Default)]
pub struct Arena {
    nodes: Vec<RawNode>,
}

impl Arena {
    fn alloc(&mut self, node: RawNode) -> NodeId {
        let id = NodeId(self.nodes.len() as u32);
        self.nodes.push(node);
        id
    }

    pub fn get(&self, id: NodeId) -> &RawNode {
        &self.nodes[id.0 as usize]
    }

    pub fn set_kind(&mut self, id: NodeId, kind: DtcgValueSyntax) {
        self.nodes[id.0 as usize].value = kind;
    }
}

pub fn build(
    arena: &mut Arena,
    doc: SourceId,
    value: &json_spanned::parser::Spanned<JsonValue>,
    parent: Option<NodeId>,
) -> NodeId {
    // Reserve the slot first so the parent gets a lower ID than its children,
    // and children can record it as their parent.
    let id = arena.alloc(RawNode {
        parent,
        span: SourceSpan::from_json_span(doc, value.span),
        value: DtcgValueSyntax::Null,
    });

    let kind = match &value.value {
        JsonValue::Object(members) => DtcgValueSyntax::Object(
            members
                .iter()
                .map(|m| DtcgMemberSyntax {
                    key: Spanned::new(
                        m.key.value.clone(),
                        SourceSpan::from_json_span(doc, m.key.span),
                    ),
                    // A member spans from the start of its key to the end of its value.
                    span: SourceSpan::new(doc, m.key.span.start, m.value.span.end),
                    value: build(arena, doc, &m.value, Some(id)),
                })
                .collect(),
        ),
        JsonValue::Array(items) => DtcgValueSyntax::Array(
            items
                .iter()
                .map(|v| build(arena, doc, &v, Some(id)))
                .collect(),
        ),
        JsonValue::String(s) => DtcgValueSyntax::String(s.clone()),
        JsonValue::Number(n) => DtcgValueSyntax::Number(*n),
        JsonValue::Boolean(b) => DtcgValueSyntax::Bool(*b),
        JsonValue::Null => DtcgValueSyntax::Null,
        JsonValue::Invalid => DtcgValueSyntax::Invalid,
    };

    arena.set_kind(id, kind);
    id
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceIdGenerator;

    fn build_from(input: &str) -> (Arena, NodeId, SourceId) {
        let (tree, errors) = json_spanned::parse_json(input);
        assert!(errors.is_empty(), "unexpected syntax errors: {errors:?}");
        let tree = tree.expect("expected a tree");
        let source = SourceIdGenerator::default().generate();
        let mut arena = Arena::default();
        let root = build(&mut arena, source, &tree, None);
        (arena, root, source)
    }

    /// Returns the child node ID stored under `key` in an object node.
    fn member(arena: &Arena, obj: NodeId, key: &str) -> NodeId {
        let DtcgValueSyntax::Object(members) = &arena.get(obj).value else {
            panic!("expected object");
        };
        members
            .iter()
            .find(|m| m.key.value == key)
            .unwrap_or_else(|| panic!("missing key {key}"))
            .value
    }

    /// Returns the whole-member span of the member stored under `key` in an object node.
    fn member_span(arena: &Arena, obj: NodeId, key: &str) -> SourceSpan {
        let DtcgValueSyntax::Object(members) = &arena.get(obj).value else {
            panic!("expected object");
        };
        members
            .iter()
            .find(|m| m.key.value == key)
            .unwrap_or_else(|| panic!("missing key {key}"))
            .span
            .clone()
    }

    fn items(arena: &Arena, arr: NodeId) -> &[NodeId] {
        let DtcgValueSyntax::Array(items) = &arena.get(arr).value else {
            panic!("expected array");
        };
        items
    }

    #[test]
    fn alloc_assigns_sequential_ids() {
        let source = SourceIdGenerator::default().generate();
        let mut arena = Arena::default();
        let node = || RawNode {
            parent: None,
            span: SourceSpan::new(source, 0, 0),
            value: DtcgValueSyntax::Null,
        };

        assert_eq!(arena.alloc(node()), NodeId(0));
        assert_eq!(arena.alloc(node()), NodeId(1));
        assert_eq!(arena.alloc(node()), NodeId(2));
        assert_eq!(arena.nodes.len(), 3);
    }

    #[test]
    fn set_kind_replaces_only_the_value() {
        let source = SourceIdGenerator::default().generate();
        let mut arena = Arena::default();
        let id = arena.alloc(RawNode {
            parent: None,
            span: SourceSpan::new(source, 2, 5),
            value: DtcgValueSyntax::Null,
        });

        arena.set_kind(id, DtcgValueSyntax::Bool(true));

        let node = arena.get(id);
        assert!(matches!(node.value, DtcgValueSyntax::Bool(true)));
        assert_eq!(node.span, SourceSpan::new(source, 2, 5));
    }

    #[test]
    fn build_primitives() {
        let cases: Vec<(&str, fn(&DtcgValueSyntax) -> bool)> = vec![
            ("null", |v| matches!(v, DtcgValueSyntax::Null)),
            ("true", |v| matches!(v, DtcgValueSyntax::Bool(true))),
            ("false", |v| matches!(v, DtcgValueSyntax::Bool(false))),
            (
                "42",
                |v| matches!(v, DtcgValueSyntax::Number(n) if n.as_u64() == Some(42)),
            ),
            (
                "-1.5",
                |v| matches!(v, DtcgValueSyntax::Number(n) if n.as_f64() == -1.5),
            ),
            (
                r#""hi""#,
                |v| matches!(v, DtcgValueSyntax::String(s) if s == "hi"),
            ),
        ];

        for (input, check) in cases {
            let (arena, root, _) = build_from(input);
            assert_eq!(arena.nodes.len(), 1, "input: {input}");
            assert!(check(&arena.get(root).value), "input: {input}");
        }
    }

    #[test]
    fn empty_containers_have_no_children() {
        let (arena, root, _) = build_from("{}");
        assert_eq!(arena.nodes.len(), 1);
        assert!(matches!(&arena.get(root).value, DtcgValueSyntax::Object(m) if m.is_empty()));

        let (arena, root, _) = build_from("[]");
        assert_eq!(arena.nodes.len(), 1);
        assert!(items(&arena, root).is_empty());
    }

    #[test]
    fn root_is_first_and_has_no_parent() {
        let (arena, root, _) = build_from(r#"{"a": {"b": [1, 2]}}"#);
        assert_eq!(root, NodeId(0));
        assert!(arena.get(root).parent.is_none());
    }

    #[test]
    fn nested_structure_allocates_one_node_per_value_in_pre_order() {
        let (arena, root, _) = build_from(r#"{"a": {"b": [1, 2]}}"#);

        // root object, "a" object, "b" array, 1, 2
        assert_eq!(arena.nodes.len(), 5);

        let a = member(&arena, root, "a");
        let b = member(&arena, a, "b");
        let arr = items(&arena, b);

        assert_eq!([a, b, arr[0], arr[1]], [NodeId(1), NodeId(2), NodeId(3), NodeId(4)]);
    }

    #[test]
    fn children_record_their_parent() {
        let (arena, root, _) = build_from(r#"{"a": {"b": [1, 2]}}"#);
        let a = member(&arena, root, "a");
        let b = member(&arena, a, "b");
        let arr = items(&arena, b).to_vec();

        assert_eq!(arena.get(a).parent, Some(root));
        assert_eq!(arena.get(b).parent, Some(a));
        for child in arr {
            assert_eq!(arena.get(child).parent, Some(b));
        }
    }

    #[test]
    fn every_non_root_parent_has_a_lower_id() {
        let (arena, _, _) = build_from(r#"{"a": [1, {"b": null}], "c": {"d": [true]}}"#);
        for (index, node) in arena.nodes.iter().enumerate() {
            if let Some(parent) = node.parent {
                assert!((parent.0 as usize) < index);
            }
        }
    }

    #[test]
    fn member_order_and_duplicate_keys_are_preserved() {
        let (arena, root, _) = build_from(r#"{"z": 1, "a": 2, "z": 3}"#);
        let DtcgValueSyntax::Object(members) = &arena.get(root).value else {
            panic!("expected object");
        };

        let keys: Vec<_> = members.iter().map(|m| m.key.value.as_str()).collect();
        assert_eq!(keys, ["z", "a", "z"]);
        assert_eq!(arena.nodes.len(), 4);

        let values: Vec<_> = members
            .iter()
            .map(|m| match &arena.get(m.value).value {
                DtcgValueSyntax::Number(n) => n.as_u64().unwrap(),
                other => panic!("expected number, got {other:?}"),
            })
            .collect();
        assert_eq!(values, [1, 2, 3]);
    }

    #[test]
    fn node_and_key_spans_point_at_the_source_text() {
        let input = r#"{"abc": [10, "x"]}"#;
        let (arena, root, source) = build_from(input);

        assert_eq!(
            arena.get(root).span,
            SourceSpan::new(source, 0, input.len())
        );

        let DtcgValueSyntax::Object(members) = &arena.get(root).value else {
            panic!("expected object");
        };
        let key_span = &members[0].key.span;
        assert_eq!(key_span.source_id, source);
        assert_eq!(&input[key_span.span.clone()], r#""abc""#);

        assert_eq!(members[0].span.source_id, source);
        assert_eq!(
            &input[members[0].span.span.clone()],
            r#""abc": [10, "x"]"#
        );

        let arr = members[0].value;
        assert_eq!(&input[arena.get(arr).span.span.clone()], r#"[10, "x"]"#);

        let elems = items(&arena, arr);
        assert_eq!(&input[arena.get(elems[0]).span.span.clone()], "10");
        assert_eq!(&input[arena.get(elems[1]).span.span.clone()], r#""x""#);
    }

    #[test]
    fn member_span_covers_key_through_value_for_every_value_kind() {
        let input = r#"{ "n" : 1, "s":"x", "o": { "k": null }, "a" :[ 1 ,2 ], "t": true }"#;
        let (arena, root, _) = build_from(input);

        let expected = [
            ("n", r#""n" : 1"#),
            ("s", r#""s":"x""#),
            ("o", r#""o": { "k": null }"#),
            ("a", r#""a" :[ 1 ,2 ]"#),
            ("t", r#""t": true"#),
        ];
        for (key, text) in expected {
            let span = member_span(&arena, root, key);
            assert_eq!(&input[span.span], text, "key: {key}");
        }
    }

    #[test]
    fn member_span_contains_key_and_value_spans() {
        let input = r#"{"a": {"b": [1, 2]}}"#;
        let (arena, root, _) = build_from(input);
        let DtcgValueSyntax::Object(members) = &arena.get(root).value else {
            panic!("expected object");
        };

        for m in members {
            let value_span = &arena.get(m.value).span.span;
            assert_eq!(m.span.span.start, m.key.span.span.start);
            assert_eq!(m.span.span.end, value_span.end);
            assert!(m.span.span.start < value_span.start);
        }
    }

    #[test]
    fn nested_member_spans_are_independent() {
        let input = r#"{"a": {"b": [1, 2]}}"#;
        let (arena, root, _) = build_from(input);
        let a = member(&arena, root, "a");

        assert_eq!(
            &input[member_span(&arena, root, "a").span],
            r#""a": {"b": [1, 2]}"#
        );
        assert_eq!(&input[member_span(&arena, a, "b").span], r#""b": [1, 2]"#);
    }

    #[test]
    fn duplicate_keys_have_distinct_member_spans() {
        let input = r#"{"z": 1, "a": 2, "z": 3}"#;
        let (arena, root, _) = build_from(input);
        let DtcgValueSyntax::Object(members) = &arena.get(root).value else {
            panic!("expected object");
        };

        let texts: Vec<_> = members
            .iter()
            .map(|m| &input[m.span.span.clone()])
            .collect();
        assert_eq!(texts, [r#""z": 1"#, r#""a": 2"#, r#""z": 3"#]);
    }

    #[test]
    fn spans_carry_the_given_source_id() {
        let mut ids = SourceIdGenerator::default();
        let _first = ids.generate();
        let second = ids.generate();

        let (tree, _) = json_spanned::parse_json(r#"{"a": 1}"#);
        let mut arena = Arena::default();
        build(&mut arena, second, &tree.unwrap(), None);

        assert!(arena.nodes.iter().all(|n| n.span.source_id == second));
        let DtcgValueSyntax::Object(members) = &arena.get(NodeId(0)).value else {
            panic!("expected object");
        };
        assert_eq!(members[0].span.source_id, second);
        assert_eq!(members[0].key.span.source_id, second);
    }

    #[test]
    fn multiple_documents_share_one_arena() {
        let mut ids = SourceIdGenerator::default();
        let (doc_a, doc_b) = (ids.generate(), ids.generate());
        let mut arena = Arena::default();

        let (tree_a, _) = json_spanned::parse_json(r#"{"a": 1}"#);
        let (tree_b, _) = json_spanned::parse_json(r#"{"b": 2}"#);
        let root_a = build(&mut arena, doc_a, &tree_a.unwrap(), None);
        let root_b = build(&mut arena, doc_b, &tree_b.unwrap(), None);

        assert_eq!((root_a, root_b), (NodeId(0), NodeId(2)));
        assert_eq!(arena.get(root_a).span.source_id, doc_a);
        assert_eq!(arena.get(root_b).span.source_id, doc_b);
        assert!(arena.get(root_b).parent.is_none());

        let b_child = member(&arena, root_b, "b");
        assert_eq!(arena.get(b_child).parent, Some(root_b));
    }

    #[test]
    fn invalid_nodes_from_recovery_are_kept() {
        let input = r#"{"ok": 1, "bad": [1, {]}"#;
        let (tree, errors) = json_spanned::parse_json(input);
        assert!(!errors.is_empty());

        let source = SourceIdGenerator::default().generate();
        let mut arena = Arena::default();
        let root = build(&mut arena, source, &tree.unwrap(), None);

        assert!(matches!(arena.get(root).value, DtcgValueSyntax::Object(_)));
        let ok = member(&arena, root, "ok");
        assert!(matches!(arena.get(ok).value, DtcgValueSyntax::Number(_)));
    }

    #[test]
    #[should_panic]
    fn get_with_out_of_range_id_panics() {
        Arena::default().get(NodeId(0));
    }
}
