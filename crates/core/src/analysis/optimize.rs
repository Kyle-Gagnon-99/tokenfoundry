//! Optimization analysis types and transforms.

use std::collections::{HashMap, HashSet};

use crate::{
    analysis::{
        TokenGraph,
        canonical::{CanonicalSetBundle, CanonicalToken},
    },
    ir::{TokenId, TokenPath},
};

#[derive(Debug, Clone)]
pub struct OptimizableToken {
    pub canonical: CanonicalToken,

    // Token metadata to relate across different structures
    pub token_id: TokenId,

    // Optimization metadata
    pub aliased_by: HashSet<TokenPath>,
    pub depends_on: HashSet<TokenPath>,
    pub origin_sets: HashSet<String>,
}

impl OptimizableToken {
    pub fn from_canonical(canonical: CanonicalToken, token_id: TokenId) -> Self {
        Self {
            canonical,
            token_id,
            aliased_by: HashSet::new(),
            depends_on: HashSet::new(),
            origin_sets: HashSet::new(),
        }
    }
}

pub fn build_optimizable_tokens(
    canonical_bundle: &CanonicalSetBundle,
    graph: &TokenGraph,
    source_set_name: &str,
) -> HashMap<TokenPath, OptimizableToken> {
    let mut opt_map: HashMap<TokenPath, OptimizableToken> = canonical_bundle
        .tokens
        .iter()
        .map(|ct| {
            (
                ct.path.clone(),
                OptimizableToken::from_canonical(ct.clone(), ct.token_id),
            )
        })
        .collect();

    for opt in opt_map.values_mut() {
        opt.origin_sets.insert(source_set_name.to_string());
    }

    let _ = &canonical_bundle.by_token_id;
    let _ = graph;
    opt_map
}
