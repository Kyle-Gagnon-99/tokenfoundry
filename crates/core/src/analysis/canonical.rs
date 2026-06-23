//! Canonical analysis types and conversions.
//!
//! This module intentionally starts as a structural placeholder so the
//! compile pipeline has a stable location for canonical token logic.

use std::collections::{HashMap, HashSet};
use std::hash::{Hash, Hasher};

use crate::ir::{
    IrDocument, IrNode, IrToken, IrTokenType, IrTokenValue, TokenId, TokenPath, TokenValue,
};

#[derive(Debug, Clone)]
pub struct CanonicalToken {
    pub token_id: TokenId,
    pub path: TokenPath,
    pub token_type: IrTokenType,
    pub value: CanonicalValue,
}

impl PartialEq for CanonicalToken {
    fn eq(&self, other: &Self) -> bool {
        self.path == other.path && self.token_type == other.token_type && self.value == other.value
    }
}

impl Eq for CanonicalToken {}

impl Hash for CanonicalToken {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.path.hash(state);
        self.token_type.hash(state);
        self.value.hash(state);
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum CanonicalValue {
    Alias { target_path: TokenPath },
    Literal(IrTokenValue),
}

#[derive(Debug, Clone)]
pub struct CanonicalSetBundle {
    pub tokens: HashSet<CanonicalToken>,
    pub by_token_id: HashMap<TokenId, CanonicalToken>,
}

#[derive(Debug, thiserror::Error)]
pub enum CanonicalizeError {
    #[error("unexpected JSON ref in canonicalization: {pointer}")]
    UnexpectedJsonRef { pointer: String },
}

impl CanonicalToken {
    pub fn from_ir_token(token: &IrToken) -> Result<Self, CanonicalizeError> {
        let value = match &token.value {
            TokenValue::Alias(alias) => CanonicalValue::Alias {
                target_path: alias.target_path.clone(),
            },
            TokenValue::Value(v) => CanonicalValue::Literal(v.clone()),
            TokenValue::Ref(r) => {
                return Err(CanonicalizeError::UnexpectedJsonRef {
                    pointer: r.pointer.to_string(),
                });
            }
        };

        Ok(Self {
            token_id: token.common.id,
            path: token.common.path.clone(),
            token_type: token.token_type,
            value,
        })
    }
}

pub fn canonical_set_from_document(
    doc: &IrDocument,
) -> Result<CanonicalSetBundle, CanonicalizeError> {
    let mut set = HashSet::new();
    let mut token_id_to_canonical = HashMap::new();
    ir_tokens_to_canonical(&doc.tokens, &mut set, &mut token_id_to_canonical)?;
    Ok(CanonicalSetBundle {
        tokens: set,
        by_token_id: token_id_to_canonical,
    })
}

/// Given a collection of IR nodes, recursively extract all tokens and convert them to their canonical form.
/// The resulting set will contain one `CanonicalToken` for every `IrToken` found in the input nodes, including those nested within groups.
///
/// # Arguments
///
/// * `nodes` - A slice of `IrNode` instances to process. This can include both `IrToken` and `IrGroup` nodes.
/// * `out` - A mutable reference to a `HashSet<CanonicalToken>` where the resulting canonical tokens will be collected. The function will insert a `CanonicalToken` for each `IrToken` found.
///
/// # Returns
///
/// * `Ok(())` if the function successfully processes all nodes and converts them to canonical tokens.
/// * `Err(CanonicalizeError)` if the function encounters an unexpected JSON reference during the conversion process.
fn ir_tokens_to_canonical(
    nodes: &[IrNode],
    out: &mut HashSet<CanonicalToken>,
    token_id_to_canonical: &mut HashMap<TokenId, CanonicalToken>,
) -> Result<(), CanonicalizeError> {
    for node in nodes {
        match node {
            IrNode::Token(t) => {
                let canonical = CanonicalToken::from_ir_token(t)?;
                token_id_to_canonical.insert(t.common.id, canonical.clone());
                out.insert(canonical);
            }
            IrNode::Group(g) => ir_tokens_to_canonical(&g.children, out, token_id_to_canonical)?,
        }
    }
    Ok(())
}

/// Given a list of sets of canonical tokens, compute the intersection of these sets, returning a new set that contains only the tokens
/// that are present in every set input list. If the input list is empty, the function will return an empty set.
/// This function is useful for finding invariants across multiple token sets, which can be used for optimization and analysis purposes.
///
/// # Arguments
///
/// * `sets` - A slice of `HashSet<CanonicalToken>` instances, where each set represents a collection of canonical tokens. The function will compute the intersection of these sets.
///
/// # Returns
///
/// * A `HashSet<CanonicalToken>` that contains only the tokens that are present in every set in the input list. If the input list is empty, the function will return an empty set.
pub fn intersection_of_canonical_sets(sets: &[HashSet<CanonicalToken>]) -> HashSet<CanonicalToken> {
    if sets.is_empty() {
        return HashSet::new();
    }

    let mut iter = sets.iter();
    let Some(first) = iter.next() else {
        return HashSet::new();
    };

    let mut acc = first.clone();
    for set in iter {
        acc = acc.intersection(set).cloned().collect();
    }
    acc
}
