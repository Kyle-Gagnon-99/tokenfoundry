//! The `pipeline` module runs the IR passes in order, so callers don't have to know the order or the gating rules.
//!
//! ```text
//! layers ─► build arena ─► merge ─► lower ─► $extends ─► (cycle gate) ─► references ─► (cycle gate) ─► types ─► Analyzed
//! ```
//!
//! The gates matter because later passes assume a finite, acyclic world. Type resolution walks "my type comes
//! from my group, or from the token I refer to", and on a cycle that walk never ends. Stopping here means the
//! next stage can recurse freely. Other errors (an unresolved alias, a bad shape) do *not* stop the pipeline:
//! reporting as many independent problems as possible in one run is better for the user, and the later stages
//! simply skip what could not be resolved.

use json_spanned::parser::{JsonValue, Spanned};

use crate::{
    diagnostics::{code::DiagnosticCode, diagnostic::DiagnosticStore},
    ir::{
        arena::{self, Arena},
        extends::expand_extends,
        lower::{Lowered, lower},
        merge::{LayerOrigin, Merged},
        refs::{References, resolve_references},
        types::{TypeMap, resolve_types},
    },
    source::SourceId,
};

/// One parsed document, in the order it should be layered: later layers win.
pub struct Layer<'a> {
    pub source: SourceId,
    pub origin: LayerOrigin,
    pub tree: &'a Spanned<JsonValue>,
}

/// Everything later stages need, and nothing that is still ambiguous: the lowered tree has `$extends` expanded and
/// both it and the reference graph are free of cycles.
#[derive(Debug)]
pub struct Analyzed {
    pub arena: Arena,
    pub lowered: Lowered,
    pub references: References,
    pub types: TypeMap,
}

/// Why the pipeline stopped before producing an [`Analyzed`]. The details are in the diagnostics.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Blocked {
    /// No layers, or nothing usable in them.
    NothingToCompile,
    /// `$extends` cycle. Reference analysis would run on a half-expanded tree, so it is skipped.
    ExtendsCycle,
    /// A reference or alias cycle.
    ReferenceCycle,
}

pub fn analyze(layers: &[Layer<'_>], diags: &mut DiagnosticStore) -> Result<Analyzed, Blocked> {
    let mut arena = Arena::default();
    let mut merged = Merged::new();

    for layer in layers {
        let root = arena::build(&mut arena, layer.source, layer.tree, None);
        merged.layer(&arena, root, layer.origin.clone(), diags);
    }

    let mut lowered = lower(&arena, &merged, diags).ok_or(Blocked::NothingToCompile)?;

    let before = diags.diagnostics().len();
    expand_extends(&arena, &mut lowered, diags);
    if emitted(diags, before, DiagnosticCode::ExtendsCycle) {
        return Err(Blocked::ExtendsCycle);
    }

    let before = diags.diagnostics().len();
    let references = resolve_references(&arena, &lowered, diags);
    if emitted(diags, before, DiagnosticCode::ReferenceCycle) {
        return Err(Blocked::ReferenceCycle);
    }

    let types = resolve_types(&arena, &lowered, &references, diags);

    Ok(Analyzed {
        arena,
        lowered,
        references,
        types,
    })
}

/// Whether a diagnostic with `code` was emitted after the first `since` ones.
fn emitted(diags: &DiagnosticStore, since: usize, code: DiagnosticCode) -> bool {
    diags.diagnostics()[since..].iter().any(|d| d.code == code)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceIdGenerator;

    fn run(docs: &[&str]) -> (Result<Analyzed, Blocked>, DiagnosticStore) {
        let mut ids = SourceIdGenerator::default();
        let trees: Vec<_> = docs
            .iter()
            .map(|d| {
                let (tree, errors) = json_spanned::parse_json(d);
                assert!(errors.is_empty());
                (ids.generate(), tree.unwrap())
            })
            .collect();
        let layers: Vec<_> = trees
            .iter()
            .map(|(source, tree)| Layer {
                source: *source,
                origin: LayerOrigin::File(*source),
                tree,
            })
            .collect();

        let mut diags = DiagnosticStore::new();
        let result = analyze(&layers, &mut diags);
        (result, diags)
    }

    #[test]
    fn a_clean_project_is_analyzed_across_layers() {
        let (result, diags) = run(&[
            r#"{"base": {"$type": "number", "a": {"$value": 1}}, "d": {"$extends": "{base}"}}"#,
            r#"{"alias": {"$value": "{d.a}"}}"#,
        ]);

        let analyzed = result.expect("no cycles");
        assert!(diags.diagnostics().is_empty());
        assert_eq!(analyzed.references.targets.len(), 1);
    }

    #[test]
    fn no_layers_is_reported_as_nothing_to_compile() {
        assert_eq!(run(&[]).0.unwrap_err(), Blocked::NothingToCompile);
    }

    #[test]
    fn an_extends_cycle_stops_before_reference_analysis() {
        let (result, diags) = run(&[
            r#"{"a": {"$extends": "{b}"}, "b": {"$extends": "{a}"}, "t": {"$value": "{nope}"}}"#,
        ]);

        assert_eq!(result.unwrap_err(), Blocked::ExtendsCycle);
        // The unresolved alias was not reported: that pass never ran.
        assert!(
            diags
                .diagnostics()
                .iter()
                .all(|d| d.code == DiagnosticCode::ExtendsCycle)
        );
    }

    #[test]
    fn a_reference_cycle_blocks() {
        let (result, _) = run(&[r#"{"a": {"$value": "{b}"}, "b": {"$value": "{a}"}}"#]);
        assert_eq!(result.unwrap_err(), Blocked::ReferenceCycle);
    }

    #[test]
    fn a_ref_token_resolves_and_records_target() {
        let (result, diags) = run(&[r##"{"target": {"$type": "number", "$value": 42}, "pointer": {"$ref": "#/target"}}"##]);

        let analyzed = result.expect("should analyze without error");
        assert!(diags.diagnostics().is_empty());
        assert_eq!(analyzed.references.targets.len(), 1);

        let pointer_token = analyzed.lowered.member(&["pointer"]).unwrap();
        let target_token = analyzed.lowered.member(&["target"]).unwrap();
        assert_eq!(analyzed.references.target(pointer_token), Some(target_token));
    }

    #[test]
    fn a_ref_inside_composite_value_resolves() {
        let (result, diags) = run(&[
            r##"{"base": {"val": {"$type": "number", "$value": 10}}, "composite": {"$type": "number", "$value": {"nested": {"$ref": "#/base/val/$value"}}}}"##,
        ]);

        let analyzed = result.expect("should analyze without error");
        assert!(diags.diagnostics().is_empty());
        assert_eq!(analyzed.references.targets.len(), 1);

        let target_val = analyzed.lowered.member(&["base", "val"]).unwrap();
        let target_value_prop = match &analyzed.lowered.get(target_val).kind {
            crate::ir::lower::PropKind::Token(t) => match t.body {
                crate::ir::lower::TokenBody::Value(v) => v,
                _ => panic!("expected value body"),
            },
            _ => panic!("expected token"),
        };

        let (&holder, &target) = analyzed.references.targets.iter().next().unwrap();
        assert_eq!(target, target_value_prop);
        assert_ne!(holder, target);
    }

    #[test]
    fn a_ref_cycle_blocks() {
        let (result, diags) = run(&[
            r##"{"a": {"$ref": "#/b"}, "b": {"$ref": "#/a"}}"##,
        ]);
        assert_eq!(result.unwrap_err(), Blocked::ReferenceCycle);
        assert_eq!(diags.diagnostics()[0].code, DiagnosticCode::ReferenceCycle);
    }

    #[test]
    fn a_ref_to_ancestor_creates_cycle_and_blocks() {
        // Referencing an ancestor container is a cycle via containment edge
        let (result, diags) = run(&[
            r##"{"group": {"token": {"$ref": "#/group"}}}"##,
        ]);
        assert_eq!(result.unwrap_err(), Blocked::ReferenceCycle);
        assert_eq!(diags.diagnostics()[0].code, DiagnosticCode::ReferenceCycle);
    }

    #[test]
    fn a_ref_cycle_formed_across_layers_blocks() {
        let (result, _) = run(&[
            r##"{"a": {"$value": 1}, "b": {"$ref": "#/a"}}"##,
            r##"{"a": {"$ref": "#/b"}}"##,
        ]);
        assert_eq!(result.unwrap_err(), Blocked::ReferenceCycle);
    }

    #[test]
    fn unresolved_ref_does_not_block_but_reports_diagnostic() {
        let (result, diags) = run(&[r##"{"a": {"$ref": "#/nonexistent"}, "b": {"$value": 1}}"##]);

        assert!(result.is_ok());
        assert!(diags.has_errors());
        assert_eq!(
            diags.diagnostics()[0].code,
            DiagnosticCode::UnresolvedReference
        );
    }

    #[test]
    fn a_cycle_formed_by_a_later_layer_is_found() {
        let (result, _) = run(&[
            r#"{"a": {"$value": 1}, "b": {"$value": "{a}"}}"#,
            r#"{"a": {"$value": "{b}"}}"#,
        ]);
        assert_eq!(result.unwrap_err(), Blocked::ReferenceCycle);
    }

    #[test]
    fn unresolved_references_do_not_block_but_are_reported() {
        let (result, diags) = run(&[r#"{"a": {"$value": "{missing}"}, "b": {"$value": 1}}"#]);

        assert!(result.is_ok());
        assert!(diags.has_errors());
        assert_eq!(
            diags.diagnostics()[0].code,
            DiagnosticCode::UnresolvedReference
        );
    }

    #[test]
    fn diagnostics_from_every_stage_are_collected() {
        let (result, diags) = run(&[
            r##"{"x": {"$value": 1, "$ref": "#/y"}, "y": {"$value": 1}}"##,
            r#"{"g": {"$extends": "{missing}"}, "t": {"$value": "{gone}"}}"#,
        ]);

        assert!(result.is_ok());
        let codes: Vec<_> = diags.diagnostics().iter().map(|d| d.code).collect();
        assert!(codes.contains(&DiagnosticCode::TokenValueAndRef));
        assert!(codes.contains(&DiagnosticCode::UnresolvedExtends));
        assert!(codes.contains(&DiagnosticCode::UnresolvedReference));
    }
}
