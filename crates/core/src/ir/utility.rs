//! The `utility` module contains utility functions for parsing and working with the IR

use crate::ir::{IrNode, IrToken, TokenPath};

/// Finds a token in the tree by its path.
/// Returns a reference to the IrToken if found, None otherwise.
///
/// # Arguments
/// * `nodes` - The root nodes to search through
/// * `path` - The TokenPath to search for
///
/// # Examples
/// ```ignore
/// use tokenfoundry_core::ir::{TokenPath, IrNode};
/// use tokenfoundry_core::ir::find_token_by_path;
///
/// // Assuming you have a parsed document with nodes
/// let path = TokenPath::from_segments(vec!["colors", "brand", "primary"]);
/// if let Some(token) = find_token_by_path(&nodes, &path) {
///     println!("Found token: {:?}", token.common.name);
/// }
/// ```
pub fn find_token_by_path<'a>(nodes: &'a [IrNode], path: &TokenPath) -> Option<&'a IrToken> {
    for node in nodes {
        match node {
            IrNode::Token(token) => {
                if token.common.path == *path {
                    return Some(token);
                }
            }
            IrNode::Group(group) => {
                // Recursively search in the group's children
                if let Some(found) = find_token_by_path(&group.children, path) {
                    return Some(found);
                }
            }
        }
    }
    None
}

/// Checks if a token exists at the given path.
///
/// # Arguments
/// * `nodes` - The root nodes to search through
/// * `path` - The TokenPath to check for
///
/// # Returns
/// `true` if a token exists at the path, `false` otherwise
///
/// # Examples
/// ```ignore
/// use tokenfoundry_core::ir::TokenPath;
/// use tokenfoundry_core::ir::token_exists;
///
/// // Assuming you have a parsed document with nodes
/// let path = TokenPath::from_segments(vec!["colors", "brand", "primary"]);
/// if token_exists(&nodes, &path) {
///     println!("Token exists!");
/// }
/// ```
pub fn token_exists(nodes: &[IrNode], path: &TokenPath) -> bool {
    find_token_by_path(nodes, path).is_some()
}

/// Finds any node (token or group) in the tree by its path.
/// Returns a reference to the IrNode if found, None otherwise.
///
/// # Arguments
/// * `nodes` - The root nodes to search through
/// * `path` - The TokenPath to search for
///
/// # Examples
/// ```ignore
/// use tokenfoundry_core::ir::TokenPath;
/// use tokenfoundry_core::ir::find_node_by_path;
///
/// // Assuming you have a parsed document with nodes
/// let path = TokenPath::from_segments(vec!["colors", "brand"]);
/// if let Some(node) = find_node_by_path(&nodes, &path) {
///     println!("Found node!");
/// }
/// ```
pub fn find_node_by_path<'a>(nodes: &'a [IrNode], path: &TokenPath) -> Option<&'a IrNode> {
    for node in nodes {
        match node {
            IrNode::Token(token) => {
                if token.common.path == *path {
                    return Some(node);
                }
            }
            IrNode::Group(group) => {
                // Check if the group itself matches the path
                if group.common.path == *path {
                    return Some(node);
                }
                // Recursively search in the group's children
                if let Some(found) = find_node_by_path(&group.children, path) {
                    return Some(found);
                }
            }
        }
    }
    None
}
