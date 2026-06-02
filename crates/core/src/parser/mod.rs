//! The `parser` module defines data structures and logic for parsing design tokens from various input formats, such as JSON

use std::collections::{BTreeMap, HashMap};

use crate::{
    DiagnosticProvenance, PROVENANCE_EXTENSION_KEY, ParserContext,
    errors::DiagnosticCode,
    ir::{
        Deprecation, IrDocument, IrGroupToken, IrNode, IrTokenType, JsonRef, ParseState,
        TokenAlias, TokenIdGenerator, TokenName,
    },
    parser::token::{parse_common_fields, parse_raw_token_to_ir_token},
};

pub mod token;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum RawNode {
    Token(RawToken),
    Group(RawGroup),
}

/// Represents the deprecation status of a token, which can be either a simple deprecation (indicated by a boolean `true`) or a deprecation with a message providing details about the deprecation (indicated by a string).
#[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub enum JsonDeprecation {
    /// A simple deprecation without additional details, represented by a boolean `true` in the input
    Simple(bool),
    /// A deprecation with a message providing details about the deprecation, represented by a string in the input
    WithMessage(String),
}

impl Into<Deprecation> for JsonDeprecation {
    fn into(self) -> Deprecation {
        match self {
            JsonDeprecation::Simple(bool_val) => Deprecation::Boolean(bool_val),
            JsonDeprecation::WithMessage(msg) => Deprecation::WithMessage(msg),
        }
    }
}

/// This struct represents the common properties of all token nodes in the token tree,
/// such a the description, depcration, and other metadata
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RawCommon {
    /// The description of the token, which can be used to provide additional context and information about the token's purpose and usage
    #[serde(rename = "$description")]
    pub description: Option<String>,

    /// The deprecation message for the token. The value may be a string providing details about the deprecation,
    /// or a boolean `true` indicating that the token is deprecated without providing additional details.
    #[serde(rename = "$deprecated")]
    pub deprecated: Option<JsonDeprecation>,

    /// A map of custom metadata properties for the token
    #[serde(rename = "$extensions")]
    pub extensions: Option<serde_json::Map<String, serde_json::Value>>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RawToken {
    #[serde(flatten)]
    pub common: RawCommon,

    #[serde(rename = "$value")]
    pub value: serde_json::Value,

    #[serde(rename = "$type")]
    pub token_type: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct RawGroup {
    #[serde(flatten)]
    pub common: RawCommon,

    #[serde(rename = "$type")]
    pub token_type: Option<String>,

    #[serde(flatten)]
    pub children: BTreeMap<String, RawNode>,
}

fn parse_node(
    node_name: &str,
    raw_node: &RawNode,
    ctx: &mut ParserContext,
    token_id_gen: &mut TokenIdGenerator,
) -> Option<IrNode> {
    let token_id = token_id_gen.generate();
    let token_name = if ctx.current_path.segments.is_empty() {
        TokenName(node_name.to_string())
    } else {
        TokenName(ctx.current_path.segments.join("."))
    };

    match raw_node {
        RawNode::Token(raw_token) => {
            let common =
                parse_common_fields(raw_common_of(raw_token), ctx, token_id, token_name.clone());
            match parse_raw_token_to_ir_token(raw_token, ctx, token_name, common) {
                ParseState::Parsed(token) => Some(IrNode::Token(token)),
                ParseState::Invalid(_) | ParseState::NoMatch => None,
            }
        }
        RawNode::Group(raw_group) => {
            let previous_type = ctx.current_type;

            if let Some(group_type) = &raw_group.token_type {
                match IrTokenType::from_str(group_type) {
                    Some(group_token_type) => ctx.set_current_type(group_token_type),
                    None => ctx.push_to_errors(
                        DiagnosticCode::InvalidTokenType,
                        format!("Invalid group token type: {}", group_type),
                        ctx.get_current_path(),
                    ),
                }
            }

            let common =
                parse_common_fields(raw_common_of_group(raw_group), ctx, token_id, token_name);
            let mut children = Vec::new();

            for (child_name, child_node) in &raw_group.children {
                ctx.push_to_current_path(child_name.clone());
                if let Some(parsed_child) = parse_node(child_name, child_node, ctx, token_id_gen) {
                    children.push(parsed_child);
                }
                ctx.pop_current_path();
            }

            ctx.current_type = previous_type;

            Some(IrNode::Group(IrGroupToken { common, children }))
        }
    }
}

fn raw_common_of(raw_token: &RawToken) -> &RawCommon {
    &raw_token.common
}

fn raw_common_of_group(raw_group: &RawGroup) -> &RawCommon {
    &raw_group.common
}

fn json_path(path: &[String]) -> String {
    if path.is_empty() {
        "/".to_string()
    } else {
        format!("/{}", path.join("/"))
    }
}

fn is_group_object(map: &serde_json::Map<String, serde_json::Value>) -> bool {
    !map.contains_key("$value")
}

fn parse_extends_reference_segments(ref_value: &str) -> Result<Vec<String>, String> {
    if let Some(alias) = TokenAlias::from_dtcg_alias(ref_value) {
        return Ok(alias.target_path.segments);
    }

    if let Some(json_ref) = JsonRef::parse(ref_value) {
        if json_ref.document.is_some() {
            return Err(format!(
                "External document references are not supported in '$extends': {}",
                ref_value
            ));
        }

        return Ok(json_ref.pointer.segments);
    }

    Err(format!(
        "Invalid '$extends' reference syntax: {}",
        ref_value
    ))
}

enum GroupReferenceDirective {
    Extends(String),
    Ref(String),
    Extensions(String),
}

fn extract_group_extension_reference(
    current_map: &serde_json::Map<String, serde_json::Value>,
    path: &[String],
    ctx: &mut ParserContext,
) -> Option<GroupReferenceDirective> {
    let mut directive_refs = Vec::new();

    if let Some(extends_value) = current_map.get("$extends") {
        match extends_value.as_str() {
            Some(value) => directive_refs.push((
                "$extends",
                GroupReferenceDirective::Extends(value.to_string()),
            )),
            None => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidReference,
                    "'$extends' must be a string reference".to_string(),
                    json_path(path),
                );
                return None;
            }
        }
    }

    if let Some(ref_value) = current_map.get("$ref") {
        match ref_value.as_str() {
            Some(value) => {
                directive_refs.push(("$ref", GroupReferenceDirective::Ref(value.to_string())))
            }
            None => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidReference,
                    "'$ref' must be a string reference".to_string(),
                    json_path(path),
                );
                return None;
            }
        }
    }

    if let Some(serde_json::Value::String(value)) = current_map.get("$extensions") {
        directive_refs.push((
            "$extensions",
            GroupReferenceDirective::Extensions(value.to_string()),
        ));
    }

    if directive_refs.len() > 1 {
        let directives = directive_refs
            .iter()
            .map(|(name, _)| *name)
            .collect::<Vec<_>>()
            .join(", ");
        ctx.push_to_errors(
            DiagnosticCode::InvalidReference,
            format!(
                "Group extension must use exactly one directive, found multiple: {}",
                directives
            ),
            json_path(path),
        );
        return None;
    }

    directive_refs.into_iter().next().map(|(_, value)| value)
}

fn navigate_to_node<'a>(
    root: &'a serde_json::Value,
    segments: &[String],
) -> Option<&'a serde_json::Value> {
    let mut current = root;

    for segment in segments {
        match current {
            serde_json::Value::Object(map) => {
                current = map.get(segment)?;
            }
            _ => return None,
        }
    }

    Some(current)
}

fn merge_group_values(base: serde_json::Value, local: serde_json::Value) -> serde_json::Value {
    match (base, local) {
        (serde_json::Value::Object(mut base_map), serde_json::Value::Object(local_map))
            if is_group_object(&base_map) && is_group_object(&local_map) =>
        {
            for (key, local_value) in local_map {
                if let Some(base_value) = base_map.remove(&key) {
                    base_map.insert(key, merge_group_values(base_value, local_value));
                } else {
                    base_map.insert(key, local_value);
                }
            }

            serde_json::Value::Object(base_map)
        }
        (_, local) => local,
    }
}

fn collect_diagnostic_provenance(
    value: &serde_json::Value,
    path: &str,
    map: &mut HashMap<String, DiagnosticProvenance>,
) {
    match value {
        serde_json::Value::Object(object) => {
            if let Some(provenance) = object
                .get("$extensions")
                .and_then(|extensions| extensions.get(PROVENANCE_EXTENSION_KEY))
            {
                if let (Some(source), Some(pointer)) = (
                    provenance.get("source").and_then(serde_json::Value::as_str),
                    provenance
                        .get("pointer")
                        .and_then(serde_json::Value::as_str),
                ) {
                    map.insert(
                        path.to_string(),
                        DiagnosticProvenance {
                            source: source.to_string(),
                            pointer: pointer.to_string(),
                        },
                    );
                }
            }

            for (key, child) in object {
                if key.starts_with('$') {
                    continue;
                }

                let child_path = if path == "/" {
                    format!("/{}", key)
                } else {
                    format!("{}/{}", path, key)
                };
                collect_diagnostic_provenance(child, &child_path, map);
            }
        }
        serde_json::Value::Array(items) => {
            for (index, item) in items.iter().enumerate() {
                let child_path = if path == "/" {
                    format!("/{}", index)
                } else {
                    format!("{}/{}", path, index)
                };
                collect_diagnostic_provenance(item, &child_path, map);
            }
        }
        _ => {}
    }
}

fn resolve_document_node(
    root: &serde_json::Value,
    current: &serde_json::Value,
    ctx: &mut ParserContext,
    path: &[String],
    extension_stack: &mut Vec<String>,
) -> Option<serde_json::Value> {
    let current_map = match current {
        serde_json::Value::Object(map) => map,
        _ => return Some(current.clone()),
    };

    if current_map.contains_key("$value") {
        return Some(current.clone());
    }

    let path_key = if path.is_empty() {
        "<root>".to_string()
    } else {
        path.join(".")
    };

    if extension_stack.contains(&path_key) {
        ctx.push_to_errors(
            DiagnosticCode::CircularReference,
            format!("Circular group extension detected at {}", path_key),
            json_path(path),
        );
        return None;
    }

    extension_stack.push(path_key.clone());

    let extension_ref = extract_group_extension_reference(current_map, path, ctx);

    let mut resolved_map = match extension_ref {
        Some(GroupReferenceDirective::Ref(ref_value)) => {
            let target_segments = match parse_extends_reference_segments(&ref_value) {
                Ok(segments) => segments,
                Err(message) => {
                    ctx.push_to_errors(DiagnosticCode::InvalidReference, message, json_path(path));
                    extension_stack.pop();
                    return None;
                }
            };

            let target = match navigate_to_node(root, &target_segments) {
                Some(target) => target,
                None => {
                    ctx.push_to_errors(
                        DiagnosticCode::UnresolvedReference,
                        format!("Unresolvable group extension target: {}", ref_value),
                        json_path(path),
                    );
                    extension_stack.pop();
                    return None;
                }
            };

            let resolved_target =
                match resolve_document_node(root, target, ctx, &target_segments, extension_stack) {
                    Some(serde_json::Value::Object(map)) => map,
                    Some(_) => serde_json::Map::new(),
                    None => {
                        extension_stack.pop();
                        return None;
                    }
                };

            resolved_target
        }
        Some(GroupReferenceDirective::Extends(ref_value))
        | Some(GroupReferenceDirective::Extensions(ref_value)) => {
            let target_segments = match parse_extends_reference_segments(&ref_value) {
                Ok(segments) => segments,
                Err(message) => {
                    ctx.push_to_errors(DiagnosticCode::InvalidReference, message, json_path(path));
                    extension_stack.pop();
                    return None;
                }
            };

            let target = match navigate_to_node(root, &target_segments) {
                Some(target) => target,
                None => {
                    ctx.push_to_errors(
                        DiagnosticCode::UnresolvedReference,
                        format!("Unresolvable group extension target: {}", ref_value),
                        json_path(path),
                    );
                    extension_stack.pop();
                    return None;
                }
            };

            let target_map = match target.as_object() {
                Some(map) if !map.contains_key("$value") => map,
                _ => {
                    ctx.push_to_errors(
                        DiagnosticCode::InvalidReferenceTarget,
                        format!(
                            "Group extension must reference a group, found token/non-group: {}",
                            ref_value
                        ),
                        json_path(path),
                    );
                    extension_stack.pop();
                    return None;
                }
            };

            let resolved_target = match resolve_document_node(
                root,
                &serde_json::Value::Object(target_map.clone()),
                ctx,
                &target_segments,
                extension_stack,
            ) {
                Some(serde_json::Value::Object(map)) => map,
                Some(_) => serde_json::Map::new(),
                None => {
                    extension_stack.pop();
                    return None;
                }
            };

            resolved_target
        }
        None => serde_json::Map::new(),
    };

    let skip_string_extensions = matches!(
        current_map.get("$extensions"),
        Some(serde_json::Value::String(_))
    );

    for (key, value) in current_map {
        if key == "$extends" || key == "$ref" || (skip_string_extensions && key == "$extensions") {
            continue;
        }

        if key.starts_with('$') && key != "$root" {
            resolved_map.insert(key.clone(), value.clone());
            continue;
        }

        let mut child_path = path.to_vec();
        child_path.push(key.clone());

        let resolved_child =
            match resolve_document_node(root, value, ctx, &child_path, extension_stack) {
                Some(resolved_child) => resolved_child,
                None => continue,
            };

        if let Some(inherited_child) = resolved_map.remove(key) {
            resolved_map.insert(
                key.clone(),
                merge_group_values(inherited_child, resolved_child),
            );
        } else {
            resolved_map.insert(key.clone(), resolved_child);
        }
    }

    extension_stack.pop();
    Some(serde_json::Value::Object(resolved_map))
}

pub fn parse_document(ctx: &mut ParserContext) -> Option<IrDocument> {
    let raw_value: serde_json::Value = match serde_json::from_str(&ctx.file_content) {
        Ok(node) => node,
        Err(e) => {
            ctx.push_to_errors(
                DiagnosticCode::Other,
                format!("Failed to parse JSON: {}", e),
                "/".to_string(),
            );
            return None;
        }
    };

    let resolved_value =
        match resolve_document_node(&raw_value, &raw_value, ctx, &[], &mut Vec::new()) {
            Some(value) => value,
            None => return None,
        };

    let mut diagnostic_provenance = HashMap::new();
    collect_diagnostic_provenance(&resolved_value, "/", &mut diagnostic_provenance);
    ctx.set_diagnostic_provenance(diagnostic_provenance);

    let root_node: RawNode = match serde_json::from_value(resolved_value) {
        Ok(node) => node,
        Err(e) => {
            ctx.push_to_errors(
                DiagnosticCode::Other,
                format!("Failed to build token document: {}", e),
                "/".to_string(),
            );
            return None;
        }
    };

    let mut token_id_gen = TokenIdGenerator::new();

    let tokens = match &root_node {
        RawNode::Token(_) => {
            ctx.push_to_current_path("$root".to_string());
            let parsed = parse_node("$root", &root_node, ctx, &mut token_id_gen);
            ctx.pop_current_path();
            parsed.into_iter().collect()
        }
        RawNode::Group(root_group) => {
            let previous_type = ctx.current_type;
            if let Some(group_type) = &root_group.token_type {
                match IrTokenType::from_str(group_type) {
                    Some(group_token_type) => ctx.set_current_type(group_token_type),
                    None => ctx.push_to_errors(
                        DiagnosticCode::InvalidTokenType,
                        format!("Invalid group token type: {}", group_type),
                        "/".to_string(),
                    ),
                }
            }

            let mut nodes = Vec::new();
            for (child_name, child_node) in &root_group.children {
                ctx.push_to_current_path(child_name.clone());
                if let Some(parsed_node) =
                    parse_node(child_name, child_node, ctx, &mut token_id_gen)
                {
                    nodes.push(parsed_node);
                }
                ctx.pop_current_path();
            }

            ctx.current_type = previous_type;
            nodes
        }
    };

    Some(IrDocument { tokens })
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    use crate::ir::find_token_by_path;

    fn sample_color_value() -> serde_json::Value {
        json!({
            "colorSpace": "srgb",
            "components": [1, 0, 0]
        })
    }

    #[test]
    fn test_raw_token_deserialization() {
        let json = json!({
            "$description": "A sample token",
            "$deprecated": "This token is deprecated",
            "$extensions": {
                "customProperty": "customValue"
            },
            "$value": sample_color_value(),
            "$type": "color"
        });

        let token: RawToken = serde_json::from_value(json).unwrap();
        assert_eq!(token.common.description.unwrap().as_str(), "A sample token");
        assert_eq!(
            token.common.deprecated.unwrap(),
            JsonDeprecation::WithMessage(String::from("This token is deprecated"))
        );
        assert_eq!(
            token
                .common
                .extensions
                .unwrap()
                .get("customProperty")
                .unwrap()
                .as_str()
                .unwrap(),
            "customValue"
        );
        assert_eq!(token.value, sample_color_value());
        assert_eq!(token.token_type.unwrap(), String::from("color"));
    }

    #[test]
    fn test_raw_group_deserialization() {
        let json = json!({
            "$description": "A sample group",
            "$deprecated": true,
            "$extensions": {
                "customProperty": "customValue"
            },
            "childToken": {
                "$value": sample_color_value(),
                "$type": "color"
            }
        });

        let group: RawGroup = serde_json::from_value(json).unwrap();
        assert_eq!(group.common.description.unwrap().as_str(), "A sample group");
        assert_eq!(
            group.common.deprecated.unwrap(),
            JsonDeprecation::Simple(true)
        );
        assert_eq!(
            group
                .common
                .extensions
                .unwrap()
                .get("customProperty")
                .unwrap()
                .as_str()
                .unwrap(),
            "customValue"
        );
        let child_token = match group.children.get("childToken").unwrap() {
            RawNode::Token(token) => token,
            _ => panic!("Expected a token node"),
        };
        assert_eq!(child_token.value, sample_color_value());
        assert_eq!(child_token.token_type.as_ref().unwrap(), "color");
        assert!(group.token_type.is_none());
    }

    #[test]
    fn test_raw_group_deserialization_with_type() {
        let json = json!({
            "$type": "color",
            "childToken": {
                "$value": sample_color_value()
            }
        });

        let group: RawGroup = serde_json::from_value(json).unwrap();
        assert_eq!(group.token_type.as_deref(), Some("color"));
    }

    #[test]
    fn test_missing_required_fields() {
        let json = json!({
            "$description": "A token missing required fields"
        });

        match serde_json::from_str::<RawToken>(json.to_string().as_str()) {
            Ok(_) => panic!("Expected deserialization to fail due to missing required fields"),
            Err(e) => assert!(e.to_string().contains("missing field `$value`")),
        }
    }

    #[test]
    fn test_json_parses_to_correct_token_type() {
        let json = json!({
            "$value": {
                "value": 16,
                "unit": "px"
            },
            "$type": "dimension"
        });

        match serde_json::from_str::<RawNode>(json.to_string().as_str()) {
            Ok(RawNode::Group(_)) => panic!("Expected a token node, but got a group node"),
            Ok(RawNode::Token(token)) => {
                assert_eq!(token.token_type.as_ref().unwrap(), "dimension");
                assert_eq!(
                    token.value,
                    json!({
                        "value": 16,
                        "unit": "px"
                    })
                );
            }
            Err(e) => panic!("Deserialization failed: {}", e),
        }

        let json = json!({
            "$description": "A token without a type field",
            "token-name": {
                "$type": "color",
                "$value": sample_color_value()
            }
        });

        match serde_json::from_str::<RawNode>(json.to_string().as_str()) {
            Ok(RawNode::Group(group)) => {
                let token_node = group.children.get("token-name").unwrap();
                match token_node {
                    RawNode::Token(token) => {
                        assert_eq!(token.token_type.as_ref().unwrap(), "color");
                        assert_eq!(token.value, sample_color_value());
                    }
                    _ => panic!("Expected a token node"),
                }
            }
            Ok(RawNode::Token(_)) => panic!("Expected a group node, but got a token node"),
            Err(e) => panic!("Deserialization failed: {}", e),
        }
    }

    #[test]
    fn test_parse_document_builds_ir_tree() {
        let json = json!({
            "brand": {
                "$type": "color",
                "primary": {
                    "$value": {
                        "colorSpace": "srgb",
                        "components": [0.2, 0.4, 1.0],
                        "hex": "#3366ff"
                    }
                }
            },
            "spacing": {
                "$type": "dimension",
                "sm": {
                    "$value": {
                        "value": 8,
                        "unit": "px"
                    }
                }
            }
        });

        let mut ctx = ParserContext::new(json.to_string());
        let document = parse_document(&mut ctx);

        assert!(document.is_some());
        assert!(ctx.errors.is_empty());

        let document = document.unwrap();
        assert_eq!(document.tokens.len(), 2);

        match &document.tokens[0] {
            IrNode::Group(group) => {
                assert_eq!(group.children.len(), 1);
                match &group.children[0] {
                    IrNode::Token(token) => {
                        assert_eq!(token.token_type, IrTokenType::Color);
                        assert_eq!(token.common.name.0, "brand.primary");
                        assert_eq!(token.common.canonical_name(), "brand.primary");
                        assert_eq!(token.common.value_json_pointer(), "#/brand/primary/$value");
                        assert!(matches!(
                            token.value,
                            crate::ir::TokenValue::Value(crate::ir::IrTokenValue::Color(_))
                        ));
                    }
                    _ => panic!("Expected child token node"),
                }
            }
            _ => panic!("Expected top-level group node"),
        }

        match &document.tokens[1] {
            IrNode::Group(group) => {
                assert_eq!(group.children.len(), 1);
                match &group.children[0] {
                    IrNode::Token(token) => {
                        assert_eq!(token.token_type, IrTokenType::Dimension);
                        assert_eq!(token.common.name.0, "spacing.sm");
                        assert_eq!(token.common.canonical_name(), "spacing.sm");
                        assert_eq!(token.common.value_json_pointer(), "#/spacing/sm/$value");
                        assert!(matches!(
                            token.value,
                            crate::ir::TokenValue::Value(crate::ir::IrTokenValue::Dimension(_))
                        ));
                    }
                    _ => panic!("Expected child token node"),
                }
            }
            _ => panic!("Expected top-level group node"),
        }
    }

    #[test]
    fn test_parse_document_supports_group_root_tokens() {
        let json = json!({
            "spacing": {
                "$type": "dimension",
                "$root": {
                    "$value": {
                        "value": 16,
                        "unit": "px"
                    }
                },
                "small": {
                    "$value": {
                        "value": 8,
                        "unit": "px"
                    }
                }
            }
        });

        let mut ctx = ParserContext::new(json.to_string());
        let document = parse_document(&mut ctx).expect("document should parse");

        assert!(
            ctx.errors.is_empty(),
            "unexpected parse errors: {:#?}",
            ctx.errors
        );

        let root_token = find_token_by_path(
            &document.tokens,
            &crate::ir::TokenPath::from_segments(["spacing", "$root"]),
        )
        .expect("$root token should exist");

        assert_eq!(root_token.common.name.0, "spacing.$root");
        assert_eq!(root_token.token_type, IrTokenType::Dimension);

        let small_token = find_token_by_path(
            &document.tokens,
            &crate::ir::TokenPath::from_segments(["spacing", "small"]),
        )
        .expect("small token should exist");

        assert_eq!(small_token.token_type, IrTokenType::Dimension);
    }

    #[test]
    fn test_parse_document_supports_group_extends_with_overrides() {
        let json = json!({
            "input": {
                "field": {
                    "width": {
                        "$type": "dimension",
                        "$value": {
                            "value": 12,
                            "unit": "rem"
                        }
                    },
                    "background": {
                        "$type": "color",
                        "$value": {
                            "colorSpace": "srgb",
                            "components": [1, 1, 1],
                            "hex": "#ffffff"
                        }
                    }
                }
            },
            "input-amount": {
                "$extends": "{input}",
                "field": {
                    "width": {
                        "$type": "dimension",
                        "$value": {
                            "value": 100,
                            "unit": "px"
                        }
                    }
                }
            }
        });

        let mut ctx = ParserContext::new(json.to_string());
        let document = parse_document(&mut ctx).expect("document should parse");

        assert!(
            ctx.errors.is_empty(),
            "unexpected parse errors: {:#?}",
            ctx.errors
        );

        let overridden_width = find_token_by_path(
            &document.tokens,
            &crate::ir::TokenPath::from_segments(["input-amount", "field", "width"]),
        )
        .expect("overridden width token should exist");
        assert_eq!(overridden_width.token_type, IrTokenType::Dimension);

        let inherited_background = find_token_by_path(
            &document.tokens,
            &crate::ir::TokenPath::from_segments(["input-amount", "field", "background"]),
        )
        .expect("inherited background token should exist");
        assert_eq!(inherited_background.token_type, IrTokenType::Color);
    }

    #[test]
    fn test_parse_document_supports_group_ref_with_overrides() {
        let json = json!({
            "base": {
                "button": {
                    "radius": {
                        "$type": "dimension",
                        "$value": {
                            "value": 4,
                            "unit": "px"
                        }
                    },
                    "$root": {
                        "$type": "dimension",
                        "$value": {
                            "value": 2,
                            "unit": "px"
                        }
                    }
                }
            },
            "large": {
                "$ref": "{base}",
                "button": {
                    "radius": {
                        "$type": "dimension",
                        "$value": {
                            "value": 8,
                            "unit": "px"
                        }
                    }
                }
            }
        });

        let mut ctx = ParserContext::new(json.to_string());
        let document = parse_document(&mut ctx).expect("document should parse");

        assert!(
            ctx.errors.is_empty(),
            "unexpected parse errors: {:#?}",
            ctx.errors
        );

        let overridden_radius = find_token_by_path(
            &document.tokens,
            &crate::ir::TokenPath::from_segments(["large", "button", "radius"]),
        )
        .expect("overridden radius token should exist");
        assert_eq!(overridden_radius.token_type, IrTokenType::Dimension);

        let inherited_root = find_token_by_path(
            &document.tokens,
            &crate::ir::TokenPath::from_segments(["large", "button", "$root"]),
        )
        .expect("inherited $root token should exist");
        assert_eq!(inherited_root.token_type, IrTokenType::Dimension);
    }

    #[test]
    fn test_parse_document_supports_group_ref_targeting_token() {
        let json = json!({
            "palette": {
                "accent": {
                    "$root": {
                        "$type": "color",
                        "$value": {
                            "colorSpace": "srgb",
                            "components": [0.2, 0.4, 1.0],
                            "hex": "#3366ff"
                        }
                    }
                }
            },
            "theme": {
                "accent": {
                    "$ref": "{palette.accent.$root}"
                }
            }
        });

        let mut ctx = ParserContext::new(json.to_string());
        let document = parse_document(&mut ctx).expect("document should parse");

        assert!(
            ctx.errors.is_empty(),
            "unexpected parse errors: {:#?}",
            ctx.errors
        );

        let aliased_root = find_token_by_path(
            &document.tokens,
            &crate::ir::TokenPath::from_segments(["theme", "accent"]),
        )
        .expect("aliased token should exist at theme.accent");

        assert_eq!(aliased_root.token_type, IrTokenType::Color);
        assert_eq!(aliased_root.common.name.0, "theme.accent");
    }

    #[test]
    fn test_parse_document_supports_group_extensions_string_with_overrides() {
        let json = json!({
            "palette": {
                "accent": {
                    "$root": {
                        "$type": "color",
                        "$value": {
                            "colorSpace": "srgb",
                            "components": [0.2, 0.4, 1.0],
                            "hex": "#3366ff"
                        }
                    },
                    "muted": {
                        "$type": "color",
                        "$value": {
                            "colorSpace": "srgb",
                            "components": [0.5, 0.6, 0.9],
                            "hex": "#8099e6"
                        }
                    }
                }
            },
            "theme": {
                "accent": {
                    "$extensions": "{palette.accent}",
                    "muted": {
                        "$type": "color",
                        "$value": {
                            "colorSpace": "srgb",
                            "components": [0.45, 0.55, 0.85],
                            "hex": "#738cd9"
                        }
                    }
                }
            }
        });

        let mut ctx = ParserContext::new(json.to_string());
        let document = parse_document(&mut ctx).expect("document should parse");

        assert!(
            ctx.errors.is_empty(),
            "unexpected parse errors: {:#?}",
            ctx.errors
        );

        let inherited_root = find_token_by_path(
            &document.tokens,
            &crate::ir::TokenPath::from_segments(["theme", "accent", "$root"]),
        )
        .expect("inherited $root token should exist");
        assert_eq!(inherited_root.token_type, IrTokenType::Color);

        let overridden_muted = find_token_by_path(
            &document.tokens,
            &crate::ir::TokenPath::from_segments(["theme", "accent", "muted"]),
        )
        .expect("overridden muted token should exist");
        assert_eq!(overridden_muted.token_type, IrTokenType::Color);
    }

    #[test]
    fn test_parse_document_reports_circular_group_extends() {
        let json = json!({
            "a": {
                "$extends": "{b}"
            },
            "b": {
                "$extends": "{a}"
            }
        });

        let mut ctx = ParserContext::new(json.to_string());
        let document = parse_document(&mut ctx).expect("document should still be returned");

        assert!(document.tokens.is_empty());
        assert!(
            ctx.errors
                .iter()
                .any(|error| error.code == DiagnosticCode::CircularReference)
        );
    }

    #[test]
    fn test_parse_document_rejects_string_color_value() {
        let json = json!({
            "brand": {
                "$type": "color",
                "primary": {
                    "$value": "#3366ff"
                }
            }
        });

        let mut ctx = ParserContext::new(json.to_string());
        let document = parse_document(&mut ctx).expect("document should still be returned");

        assert_eq!(document.tokens.len(), 1);

        match &document.tokens[0] {
            IrNode::Group(group) => assert!(group.children.is_empty()),
            _ => panic!("Expected top-level group node"),
        }

        assert!(!ctx.errors.is_empty());
        assert!(ctx.errors.iter().any(|error| {
            error.path == "/brand/primary/$value"
                && error.message.contains("Expected an object for color token")
        }));
    }

    #[test]
    fn test_parse_document_reports_json_errors() {
        let mut ctx = ParserContext::new("{ this is invalid json }".to_string());

        let document = parse_document(&mut ctx);
        assert!(document.is_none());
        assert!(!ctx.errors.is_empty());
    }

    #[test]
    fn test_parse_document_maps_diagnostics_to_provenance_source() {
        let json = json!({
            "semantic": {
                "background": {
                    "canvas": {
                        "$extensions": {
                            PROVENANCE_EXTENSION_KEY: {
                                "source": "themes/dark.tokens.json",
                                "pointer": "#/semantic/background/canvas"
                            }
                        },
                        "$type": "color",
                        "$value": "#3366ff"
                    }
                }
            }
        });

        let mut ctx = ParserContext::new(json.to_string());
        let _ = parse_document(&mut ctx).expect("document should still be returned");

        let mapped = ctx.errors.iter().any(|error| {
            error.file_path.as_deref() == Some("themes/dark.tokens.json")
                && error.path == "#/semantic/background/canvas/$value"
                && error.message.contains("Expected an object for color token")
        });

        assert!(
            mapped,
            "expected at least one mapped diagnostic: {:#?}",
            ctx.errors
        );
    }
}
