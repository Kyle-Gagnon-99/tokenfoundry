//! End-to-end: resolver documents and token files from disk, through the public `DtcgCompiler` API.
//! Only this test reads files; `core` itself is given contents through a provider.

use std::path::PathBuf;

use serde_json::{Value, json};
use tokenfoundry_core::{
    compiler::DtcgCompiler,
    diagnostics::code::DiagnosticCode,
    resolver::{Input, ProviderError, SourceProvider},
};

struct DirProvider(PathBuf);

impl SourceProvider for DirProvider {
    fn read(&self, path: &str) -> Result<String, ProviderError> {
        std::fs::read_to_string(self.0.join(path)).map_err(|e| match e.kind() {
            std::io::ErrorKind::NotFound => ProviderError::NotFound,
            _ => ProviderError::Unreadable(e.to_string()),
        })
    }
}

fn run(fixture: &str, input: Input) -> (Result<Value, ()>, DtcgCompiler) {
    let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/resources")
        .join(fixture);
    let text = std::fs::read_to_string(dir.join("resolver.json")).unwrap();
    let mut compiler = DtcgCompiler::new();
    let result = compiler
        .compile("resolver.json", &text, &DirProvider(dir), &input)
        .map(|tokens| serde_json::to_value(tokens).unwrap())
        .map_err(|_| ());
    (result, compiler)
}

fn token<'a>(tokens: &'a Value, path: &str) -> &'a Value {
    let want: Vec<&str> = path.split('.').collect();
    tokens
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["path"] == json!(want))
        .unwrap_or_else(|| panic!("no token `{path}` in {tokens}"))
}

fn value<'a>(tokens: &'a Value, path: &str) -> &'a Value {
    &token(tokens, path)["value"]["value"]
}

#[test]
fn light_is_the_default_theme() {
    let (result, compiler) = run("resolver_integration", Input::new());
    let tokens = result.expect("compile failed");
    assert!(!compiler.diagnostics().has_errors());

    assert_eq!(tokens.as_array().unwrap().len(), 6);
    assert_eq!(
        value(&tokens, "foundation.colors.primary")["components"],
        json!([0.2, 0.4, 1.0])
    );
    assert_eq!(
        value(&tokens, "foundation.colors.secondary")["alpha"],
        json!(0.85)
    );
    assert_eq!(
        value(&tokens, "foundation.spacing.md"),
        &json!({"value": 16.0, "unit": "px"})
    );
    assert_eq!(
        value(&tokens, "semantic.background.canvas")["components"],
        json!([1.0, 1.0, 1.0])
    );
}

#[test]
fn input_selects_the_dark_context() {
    let (result, _) = run("resolver_integration", Input::new().with("theme", "dark"));
    let tokens = result.expect("compile failed");
    assert_eq!(
        value(&tokens, "semantic.background.canvas")["components"],
        json!([0.1, 0.1, 0.1])
    );
    // Foundation tokens come from a set and are unaffected by the modifier.
    assert_eq!(
        value(&tokens, "foundation.colors.primary")["components"],
        json!([0.2, 0.4, 1.0])
    );
}

#[test]
fn contract_shape_of_a_token() {
    let (result, _) = run("resolver_integration", Input::new());
    let tokens = result.unwrap();
    let t = token(&tokens, "foundation.spacing.xs");
    assert_eq!(t["tokenType"], "dimension");
    assert_eq!(t["value"]["type"], "dimension");
    assert_eq!(t["aliasOf"], Value::Null);
}

#[test]
fn input_matching_ignores_case() {
    let (result, _) = run("resolver_integration", Input::new().with("THEME", "Dark"));
    let tokens = result.unwrap();
    assert_eq!(
        value(&tokens, "semantic.content.primary")["components"],
        json!([1.0, 1.0, 1.0])
    );
}

#[test]
fn unknown_context_fails_with_a_diagnostic() {
    let (result, compiler) = run("resolver_integration", Input::new().with("theme", "neon"));
    assert!(result.is_err());
    assert_eq!(
        compiler.diagnostics().diagnostics()[0].code,
        DiagnosticCode::InvalidResolverInput
    );
}

#[test]
fn fragment_refs_and_cross_document_property_refs_resolve() {
    // Default is dark: its tokens use `$ref` to primitives in another file, and a `{alias}` for line height.
    let (result, compiler) = run("resolver_integration_complex", Input::new());
    let tokens = result.unwrap_or_else(|_| panic!("{:?}", compiler.diagnostics().diagnostics()));

    assert_eq!(value(&tokens, "semantic.accent")["alpha"], json!(0.72));

    let body = value(&tokens, "semantic.body");
    assert_eq!(body["fontFamily"], json!(["Inter", "Arial", "sans-serif"]));
    assert_eq!(body["fontSize"], json!({"value": 16.0, "unit": "px"}));
    assert_eq!(body["fontWeight"], json!(700.0));
    assert_eq!(body["lineHeight"], json!(1.5));

    // Only the selected context's tokens exist.
    assert!(
        tokens
            .as_array()
            .unwrap()
            .iter()
            .all(|t| t["path"] != json!(["themes", "light", "semantic", "accent"]))
    );
}

#[test]
fn the_other_context_of_the_complex_fixture() {
    let (result, _) = run(
        "resolver_integration_complex",
        Input::new().with("theme", "light"),
    );
    let tokens = result.unwrap();
    assert_eq!(
        value(&tokens, "semantic.accent")["components"],
        json!([0.95, 0.95, 0.95])
    );
    assert!(
        tokens
            .as_array()
            .unwrap()
            .iter()
            .all(|t| t["path"] != json!(["semantic", "body"]))
    );
}

#[test]
fn missing_file_is_reported() {
    let dir =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/resources/resolver_integration");
    let text = std::fs::read_to_string(dir.join("resolver.json")).unwrap();
    let mut compiler = DtcgCompiler::new();
    let empty = DirProvider(dir.join("does-not-exist"));
    let result = compiler.compile("resolver.json", &text, &empty, &Input::new());
    assert!(result.is_err());
    assert!(
        compiler
            .diagnostics()
            .diagnostics()
            .iter()
            .any(|d| d.code == DiagnosticCode::FileNotFound)
    );
}
