//! Invariant detection across resolved token sets.

use std::collections::{BTreeSet, HashSet};

use crate::analysis::canonical::CanonicalToken;

/// Fully-resolved token set for a specific context combination.
///
/// Example selections:
/// - [("mode", "light"), ("contrast", "high")]
#[derive(Debug, Clone)]
pub struct ContextVariantTokens {
    pub selections: Vec<(String, String)>,
    pub tokens: HashSet<CanonicalToken>,
}

/// Invariants for a scope plus the delta from the nearest parent scope.
#[derive(Debug, Clone)]
pub struct InvariantLayer {
    pub scope: Vec<(String, String)>,
    pub invariants: HashSet<CanonicalToken>,
    pub delta_from_parent: HashSet<CanonicalToken>,
}

/// Compute hierarchical invariant layers from broadest scope to most specific.
///
/// The empty scope (`[]`) represents global invariants.
pub fn build_invariant_layers(variants: &[ContextVariantTokens]) -> Vec<InvariantLayer> {
    if variants.is_empty() {
        return Vec::new();
    }

    let mut all_scopes: BTreeSet<Vec<(String, String)>> = BTreeSet::new();
    all_scopes.insert(Vec::new());

    for variant in variants {
        let normalized = normalize_scope(&variant.selections);
        for scope in subset_scopes(&normalized) {
            all_scopes.insert(scope);
        }
    }

    let mut computed: Vec<InvariantLayer> = Vec::new();

    for scope in all_scopes {
        let matching: Vec<&ContextVariantTokens> = variants
            .iter()
            .filter(|variant| scope_matches_variant(&scope, &variant.selections))
            .collect();

        if matching.is_empty() {
            continue;
        }

        let invariants = matching
            .iter()
            .map(|variant| &variant.tokens)
            .cloned()
            .reduce(|acc, next| acc.intersection(&next).cloned().collect())
            .unwrap_or_default();

        let parent_scope = find_parent_scope(&scope, &computed);
        let parent_invariants = parent_scope
            .and_then(|parent| computed.iter().find(|layer| layer.scope == parent))
            .map(|layer| layer.invariants.clone())
            .unwrap_or_default();

        let delta_from_parent = invariants
            .difference(&parent_invariants)
            .cloned()
            .collect::<HashSet<_>>();

        computed.push(InvariantLayer {
            scope,
            invariants,
            delta_from_parent,
        });
    }

    computed
}

fn normalize_scope(scope: &[(String, String)]) -> Vec<(String, String)> {
    let mut normalized = scope.to_vec();
    normalized.sort();
    normalized
}

fn subset_scopes(scope: &[(String, String)]) -> Vec<Vec<(String, String)>> {
    let n = scope.len();
    let mut subsets = Vec::with_capacity(1usize << n);

    for mask in 0..(1usize << n) {
        let mut subset = Vec::new();
        for (idx, pair) in scope.iter().enumerate() {
            if (mask & (1usize << idx)) != 0 {
                subset.push(pair.clone());
            }
        }
        subsets.push(subset);
    }

    subsets
}

fn scope_matches_variant(scope: &[(String, String)], variant_scope: &[(String, String)]) -> bool {
    let normalized_variant = normalize_scope(variant_scope);
    scope
        .iter()
        .all(|pair| normalized_variant.iter().any(|candidate| candidate == pair))
}

fn find_parent_scope(
    scope: &[(String, String)],
    layers: &[InvariantLayer],
) -> Option<Vec<(String, String)>> {
    if scope.is_empty() {
        return None;
    }

    let mut candidates: Vec<Vec<(String, String)>> = layers
        .iter()
        .map(|layer| layer.scope.clone())
        .filter(|candidate| candidate.len() < scope.len())
        .filter(|candidate| candidate.iter().all(|pair| scope.contains(pair)))
        .collect();

    candidates.sort_by(|a, b| b.len().cmp(&a.len()).then_with(|| a.cmp(b)));
    candidates.into_iter().next()
}
