//! The `ir` module in `core` provides the intermediate representation (IR) for the token-shift project.
//! This IR is a wrapper against the parsed JSON from the `json_spanned` crate. It provides additional metadata, used
//! for further processing and transformation within the token-shift project.
//! The IR serves as an intermediate layer between the raw JSON input and the final output formats, enabling more flexible and powerful transformations.

pub mod arena;
pub mod extends;
pub mod lower;
pub mod merge;
pub mod node;
pub mod pipeline;
pub mod refs;
pub mod types;

/// Runs the pipeline up to lowering on a list of documents, for tests in the passes that follow it.
#[cfg(test)]
pub(crate) fn lower_docs(
    docs: &[&str],
) -> (
    arena::Arena,
    lower::Lowered,
    crate::diagnostics::diagnostic::DiagnosticStore,
) {
    use crate::source::SourceIdGenerator;

    let mut arena = arena::Arena::default();
    let mut ids = SourceIdGenerator::default();
    let mut merged = merge::Merged::new();
    let mut diags = crate::diagnostics::diagnostic::DiagnosticStore::new();

    for doc in docs {
        let (tree, errors) = json_spanned::parse_json(doc);
        assert!(errors.is_empty(), "syntax errors: {errors:?}");
        let source = ids.generate();
        let root = arena::build(&mut arena, source, &tree.unwrap(), None);
        merged.layer(&arena, root, merge::LayerOrigin::File(source), &mut diags);
    }

    let lowered = lower::lower(&arena, &merged, &mut diags).expect("something was layered");
    (arena, lowered, diags)
}
