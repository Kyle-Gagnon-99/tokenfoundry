//! The `id` module in `core` provides the definitions for node identifiers and the IR nodes.

use json_spanned::parser::JsonNumber;

use crate::source::{SourceSpan, Spanned};

/// NodeId represents a unique identifier for a node within the IR.
/// A node is an element within the IR, which can represent a raw JSON value or a structured collection of other nodes.
/// The NodeId is used to uniquely identify each node within the IR, ensuring that nodes can be referenced and manipulated reliably.
/// It allows us to use the NodeId as a reference without needing the data of the node itself.
/// This is useful for deep merging operations, re-mapping, and more.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Hash)]
pub struct NodeId(pub u32);

#[derive(Debug)]
pub enum DtcgValueSyntax {
    Null,
    Bool(bool),
    Number(JsonNumber),
    String(String),
    Array(Vec<NodeId>),
    Object(Vec<DtcgMemberSyntax>),
    Invalid,
}

#[derive(Debug)]
pub struct DtcgMemberSyntax {
    pub key: Spanned<String>,
    pub value: NodeId,
    pub span: SourceSpan,
}

#[derive(Debug)]
pub struct RawNode {
    pub parent: Option<NodeId>,
    pub span: SourceSpan,
    pub value: DtcgValueSyntax,
}

impl RawNode {
    /// A node is token-like if it is an object with a `$value` or `$ref` member.
    /// This is a purely syntactic check: malformed tokens (both members, wrong value shapes) are still token-like
    /// and are reported later during lowering. It is only meant for the token tree, since `$ref` also appears in
    /// resolver reference objects.
    pub fn is_token_like(&self) -> bool {
        matches!(&self.value, DtcgValueSyntax::Object(members)
        if members.iter().any(|m| matches!(m.key.value.as_str(), "$value" | "$ref")))
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        ir::arena::{Arena, build},
        source::SourceIdGenerator,
    };

    fn root_is_token_like(json: &str) -> bool {
        let (tree, errors) = json_spanned::parse_json(json);
        let tree = tree.expect("expected a tree");
        assert!(errors.is_empty(), "unexpected errors: {errors:?}");
        let source = SourceIdGenerator::default().generate();
        let mut arena = Arena::default();
        let root = build(&mut arena, source, &tree, None);
        arena.get(root).is_token_like()
    }

    #[test]
    fn value_member_is_token_like() {
        assert!(root_is_token_like(r#"{"$value": 1}"#));
        assert!(root_is_token_like(r#"{"$value": "{a.b}"}"#));
        assert!(root_is_token_like(r#"{"$type": "color", "$value": {}}"#));
    }

    #[test]
    fn ref_member_is_token_like() {
        assert!(root_is_token_like(r##"{"$ref": "#/a/b/$value"}"##));
    }

    #[test]
    fn malformed_tokens_are_still_token_like() {
        assert!(root_is_token_like(r##"{"$value": 1, "$ref": "#/a"}"##));
        assert!(root_is_token_like(r#"{"$ref": 1}"#));
        assert!(root_is_token_like(r#"{"$value": 1, "$value": 2}"#));
        assert!(root_is_token_like(r#"{"$value": null}"#));
    }

    #[test]
    fn groups_are_not_token_like() {
        assert!(!root_is_token_like("{}"));
        assert!(!root_is_token_like(r#"{"a": 1}"#));
        assert!(!root_is_token_like(r#"{"$type": "color"}"#));
        assert!(!root_is_token_like(r#"{"$description": "d", "$extends": "{a}"}"#));
    }

    #[test]
    fn child_tokens_do_not_make_the_parent_token_like() {
        assert!(!root_is_token_like(r#"{"a": {"$value": 1}}"#));
        assert!(!root_is_token_like(r#"{"$root": {"$value": 1}, "b": {"$value": 2}}"#));
    }

    #[test]
    fn key_match_is_exact() {
        assert!(!root_is_token_like(r#"{"value": 1}"#));
        assert!(!root_is_token_like(r#"{"ref": "x"}"#));
        assert!(!root_is_token_like(r#"{"$Value": 1}"#));
        assert!(!root_is_token_like(r#"{"$values": 1}"#));
        assert!(!root_is_token_like(r#"{"$value ": 1}"#));
    }

    #[test]
    fn escaped_keys_are_matched_after_unescaping() {
        assert!(root_is_token_like(r#"{"\u0024value": 1}"#));
    }

    #[test]
    fn non_objects_are_not_token_like() {
        assert!(!root_is_token_like("[1, 2]"));
        assert!(!root_is_token_like(r#"["$value"]"#));
        assert!(!root_is_token_like(r#""$value""#));
        assert!(!root_is_token_like("1"));
        assert!(!root_is_token_like("true"));
        assert!(!root_is_token_like("null"));
    }

    #[test]
    fn invalid_nodes_are_not_token_like() {
        let (tree, errors) = json_spanned::parse_json(r#"[{]"#);
        assert!(!errors.is_empty());
        let source = SourceIdGenerator::default().generate();
        let mut arena = Arena::default();
        let root = build(&mut arena, source, &tree.unwrap(), None);
        assert!(!arena.get(root).is_token_like());
    }
}
