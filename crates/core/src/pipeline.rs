//! Compiler pipeline entry points.
//!
//! The pipeline is the top-level lifecycle coordinator:
//! input resolution -> parsing -> analysis -> optimization -> output.

use crate::{
    analysis::{
        TokenGraph,
        canonical::{CanonicalToken, canonical_set_from_document},
    },
    input::{ResolveOptions, ResolvedGraphOutput, Resolver, ResolverError},
    ir::ResolutionInput,
    output::EmittableToken,
};

/// Request data for pipeline graph resolution.
#[derive(Debug, Clone)]
pub struct ResolvePipelineRequest {
    pub input: ResolutionInput,
    pub options: ResolveOptions,
}

impl ResolvePipelineRequest {
    pub fn with_defaults(input: ResolutionInput) -> Self {
        Self {
            input,
            options: ResolveOptions::default(),
        }
    }
}

/// Result shape for the resolve->parse->graph phase.
#[derive(Debug, Clone)]
pub struct PipelineGraphOutput {
    pub resolved_tokens: serde_json::Value,
    pub document: crate::ir::IrDocument,
    pub graph: TokenGraph,
    pub parser_diagnostics: Vec<crate::errors::Diagnostic>,
    pub graph_diagnostics: Vec<crate::errors::Diagnostic>,
}

/// Finalized output shape for emitters.
///
/// This output guarantees token values are lowered to either concrete
/// literals or aliases (no JSON references).
#[derive(Debug, Clone)]
pub struct PipelineFinalizedOutput {
    pub resolved_tokens: serde_json::Value,
    pub document: crate::ir::IrDocument,
    pub graph: TokenGraph,
    pub parser_diagnostics: Vec<crate::errors::Diagnostic>,
    pub graph_diagnostics: Vec<crate::errors::Diagnostic>,
    pub canonical_tokens: Vec<CanonicalToken>,
    pub emittable_tokens: Vec<EmittableToken>,
}

#[derive(Debug, thiserror::Error)]
pub enum PipelineFinalizeError {
    #[error(transparent)]
    Resolve(#[from] ResolverError),
    #[error(transparent)]
    Canonicalize(#[from] crate::analysis::canonical::CanonicalizeError),
    #[error(transparent)]
    Lower(#[from] crate::output::LoweringError),
}

impl From<ResolvedGraphOutput> for PipelineGraphOutput {
    fn from(value: ResolvedGraphOutput) -> Self {
        Self {
            resolved_tokens: value.resolved_tokens,
            document: value.document,
            graph: value.graph,
            parser_diagnostics: value.parser_diagnostics,
            graph_diagnostics: value.graph_diagnostics,
        }
    }
}

/// Executes the input-resolution through graph-analysis lifecycle.
pub fn resolve_to_graph_pipeline(
    resolver: &Resolver,
    request: ResolvePipelineRequest,
) -> Result<PipelineGraphOutput, ResolverError> {
    resolver
        .resolve_to_graph_with_options(request.input, request.options)
        .map(Into::into)
}

/// Executes the full input-resolution through finalized-token lifecycle.
///
/// This stage enforces property ref materialization and token-level ref to
/// alias conversion so downstream emitters receive only literals/aliases.
pub fn resolve_to_finalized_pipeline(
    resolver: &Resolver,
    request: ResolvePipelineRequest,
) -> Result<PipelineFinalizedOutput, PipelineFinalizeError> {
    let mut options = request.options;
    options.materialize_aliases = false;
    options.materialize_property_refs = true;
    options.convert_token_refs_to_aliases = true;

    let graph_output = resolver.resolve_to_graph_with_options(request.input, options)?;

    let canonical = canonical_set_from_document(&graph_output.document)?;
    let mut canonical_tokens: Vec<CanonicalToken> = canonical.tokens.into_iter().collect();
    canonical_tokens.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.token_id.cmp(&right.token_id))
    });

    let emittable_tokens = canonical_tokens
        .iter()
        .map(EmittableToken::from_canonical)
        .collect::<Result<Vec<_>, _>>()?;

    Ok(PipelineFinalizedOutput {
        resolved_tokens: graph_output.resolved_tokens,
        document: graph_output.document,
        graph: graph_output.graph,
        parser_diagnostics: graph_output.parser_diagnostics,
        graph_diagnostics: graph_output.graph_diagnostics,
        canonical_tokens,
        emittable_tokens,
    })
}
