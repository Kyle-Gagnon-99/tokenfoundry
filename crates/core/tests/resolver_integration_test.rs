use std::{
    fs,
    path::PathBuf,
    time::{SystemTime, UNIX_EPOCH},
};

use test_log::test;
use tokenfoundry_core::{
    ParserContext,
    analysis::graph::{EdgeKind, TokenGraph},
    input::resolver::{ResolveOptions, Resolver},
    ir::{
        IrTokenType, IrTokenValue, RefAliasOrLiteral, RefOrLiteral, ResolutionInput, TokenPath,
        TokenValue, find_token_by_path,
    },
    output::EmittableValue,
    parsing::parse_document,
    pipeline::{ResolvePipelineRequest, resolve_to_finalized_pipeline},
};
use tracing::info;

fn resolver_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/resources/resolver_integration/resolver.json")
}

fn complex_resolver_fixture_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/resources/resolver_integration_complex/resolver.json")
}

fn extract_provenance(token: &tokenfoundry_core::ir::IrToken) -> (&str, &str) {
    let provenance = token
        .common
        .extensions
        .as_ref()
        .and_then(|extensions| extensions.get("tokenfoundry.dev/provenance"))
        .expect("token should carry provenance metadata");

    let source = provenance
        .get("source")
        .and_then(serde_json::Value::as_str)
        .expect("provenance source must be a string");
    let pointer = provenance
        .get("pointer")
        .and_then(serde_json::Value::as_str)
        .expect("provenance pointer must be a string");

    (source, pointer)
}

fn strip_provenance_extensions(value: &mut serde_json::Value) {
    match value {
        serde_json::Value::Object(map) => {
            if let Some(serde_json::Value::Object(extensions)) = map.get_mut("$extensions") {
                extensions.remove(tokenfoundry_core::PROVENANCE_EXTENSION_KEY);
                if extensions.is_empty() {
                    map.remove("$extensions");
                }
            }

            let keys: Vec<String> = map.keys().cloned().collect();
            for key in keys {
                if let Some(child) = map.get_mut(&key) {
                    strip_provenance_extensions(child);
                }
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                strip_provenance_extensions(item);
            }
        }
        _ => {}
    }
}

fn unique_temp_dir(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("{}-{}", prefix, nanos))
}

#[test]
fn resolves_external_files_and_parses_typed_tokens_end_to_end() {
    let resolver =
        Resolver::from_file(resolver_fixture_path()).expect("resolver fixture should load");

    let mut input = ResolutionInput::new();
    input.add_selection("theme".to_string(), "dark".to_string());

    let resolved = resolver
        .resolve(input)
        .expect("resolver should merge external files");

    assert_eq!(
        resolved["foundation"]["colors"]["primary"]["$value"]["hex"],
        "#3366ff"
    );
    assert_eq!(
        resolved["foundation"]["spacing"]["md"]["$value"]["unit"],
        "px"
    );
    assert_eq!(
        resolved["semantic"]["background"]["canvas"]["$value"]["hex"],
        "#1a1a1a"
    );
    assert_eq!(
        resolved["semantic"]["content"]["primary"]["$value"]["hex"],
        "#ffffff"
    );

    let mut parser_context = ParserContext::new(resolved.to_string());

    let document = parse_document(&mut parser_context).expect("resolved JSON should parse into IR");

    assert!(
        parser_context.errors.is_empty(),
        "unexpected parse errors: {:#?}",
        parser_context.errors
    );

    let primary_color = find_token_by_path(
        &document.tokens,
        &TokenPath::from_segments(["foundation", "colors", "primary"]),
    )
    .expect("primary color token should exist");
    assert_eq!(primary_color.token_type, IrTokenType::Color);
    assert!(matches!(primary_color.value, TokenValue::Value(_)));

    let spacing_md = find_token_by_path(
        &document.tokens,
        &TokenPath::from_segments(["foundation", "spacing", "md"]),
    )
    .expect("spacing token should exist");
    assert_eq!(spacing_md.token_type, IrTokenType::Dimension);
    assert!(matches!(spacing_md.value, TokenValue::Value(_)));

    let dark_background = find_token_by_path(
        &document.tokens,
        &TokenPath::from_segments(["semantic", "background", "canvas"]),
    )
    .expect("dark theme background token should exist");
    assert_eq!(dark_background.token_type, IrTokenType::Color);
    assert!(matches!(dark_background.value, TokenValue::Value(_)));
}

#[test]
fn preserves_nested_json_pointer_refs_and_materializes_dtcg_aliases_after_multifile_resolution() {
    let resolver = Resolver::from_file(complex_resolver_fixture_path())
        .expect("complex resolver fixture should load");

    let mut input = ResolutionInput::new();
    input.add_selection("theme".to_string(), "dark".to_string());

    let resolved = resolver
        .resolve(input)
        .expect("complex resolver should merge external files");

    assert_eq!(resolved["semantic"]["accent"]["$value"]["alpha"], 0.72);
    assert_eq!(resolved["semantic"]["body"]["$value"]["lineHeight"], 1.5);
    assert_eq!(
        resolved["semantic"]["body"]["$value"]["fontFamily"],
        serde_json::json!(["Inter", "Arial", "sans-serif"])
    );
    assert_eq!(
        resolved["foundation"]["brand"]["primary"]["$value"]["hex"],
        "#3366ff"
    );

    let mut parser_context = ParserContext::new(resolved.to_string());

    let document =
        parse_document(&mut parser_context).expect("complex resolved JSON should parse into IR");

    println!("Parsed IR document:\n{:#?}", document);

    assert!(
        parser_context.errors.is_empty(),
        "unexpected parse errors: {:#?}",
        parser_context.errors
    );

    let accent = find_token_by_path(
        &document.tokens,
        &TokenPath::from_segments(["semantic", "accent"]),
    )
    .expect("semantic accent token should exist");
    assert_eq!(accent.token_type, IrTokenType::Color);

    match &accent.value {
        TokenValue::Value(IrTokenValue::Color(color)) => {
            assert!(matches!(color.alpha, Some(RefOrLiteral::Literal(_))));
        }
        other => panic!("expected parsed color token value, got {:?}", other),
    }

    let body = find_token_by_path(
        &document.tokens,
        &TokenPath::from_segments(["semantic", "body"]),
    )
    .expect("semantic body token should exist");
    assert_eq!(body.token_type, IrTokenType::Typography);

    match &body.value {
        TokenValue::Value(IrTokenValue::Typography(typography)) => {
            assert!(matches!(
                typography.font_family.0,
                RefAliasOrLiteral::Literal(_)
            ));
            assert!(matches!(
                typography.font_size.0,
                RefAliasOrLiteral::Literal(_)
            ));
            assert!(matches!(
                typography.line_height.0,
                RefAliasOrLiteral::Literal(_)
            ));
        }
        other => panic!("expected parsed typography token value, got {:?}", other),
    }
}

#[test]
fn complex_resolution_produces_expected_merged_tree_shape() {
    let resolver = Resolver::from_file(complex_resolver_fixture_path())
        .expect("complex resolver fixture should load");

    let mut input = ResolutionInput::new();
    input.add_selection("theme".to_string(), "dark".to_string());

    let resolved = resolver
        .resolve(input)
        .expect("complex resolver should merge external files");

    info!(
        "Resolved output with provenance extensions:\n{}",
        serde_json::to_string_pretty(&resolved).unwrap()
    );

    let mut normalized_resolved = resolved.clone();
    strip_provenance_extensions(&mut normalized_resolved);

    let expected = serde_json::json!({
        "primitives": {
            "opacity": {
                "$type": "number",
                "strong": { "$value": 0.72 }
            },
            "typography": {
                "bodyFamily": {
                    "$type": "fontFamily",
                    "$value": ["Inter", "Arial", "sans-serif"]
                },
                "bodyLineHeight": {
                    "$type": "number",
                    "$value": 1.5
                }
            },
            "spacing": {
                "$type": "dimension",
                "md": { "$value": { "value": 16, "unit": "px" } }
            }
        },
        "foundation": {
            "brand": {
                "$type": "color",
                "primary": {
                    "$value": {
                        "colorSpace": "srgb",
                        "components": [0.2, 0.4, 1.0],
                        "alpha": 1,
                        "hex": "#3366ff"
                    }
                }
            }
        },
        "semantic": {
            "accent": {
                "$type": "color",
                "$value": {
                    "colorSpace": "srgb",
                    "components": [0.2, 0.4, 1.0],
                    "alpha": 0.72,
                    "hex": "#3366ff"
                }
            },
            "body": {
                "$type": "typography",
                "$value": {
                    "fontFamily": ["Inter", "Arial", "sans-serif"],
                    "fontSize": { "value": 16, "unit": "px" },
                    "fontWeight": 700,
                    "letterSpacing": { "value": 0, "unit": "px" },
                    "lineHeight": 1.5
                }
            }
        }
    });

    assert_eq!(normalized_resolved, expected);

    info!(
        "Resolved output:\n{}",
        serde_json::to_string_pretty(&resolved).unwrap()
    );
}

#[test]
fn preserves_external_source_provenance_in_parsed_ir_extensions() {
    let resolver =
        Resolver::from_file(resolver_fixture_path()).expect("resolver fixture should load");

    let mut input = ResolutionInput::new();
    input.add_selection("theme".to_string(), "dark".to_string());

    let resolved = resolver
        .resolve(input)
        .expect("resolver should merge external files");

    let mut parser_context = ParserContext::new(resolved.to_string());
    let document = parse_document(&mut parser_context).expect("resolved JSON should parse into IR");

    assert!(
        parser_context.errors.is_empty(),
        "unexpected parse errors: {:#?}",
        parser_context.errors
    );

    let primary_color = find_token_by_path(
        &document.tokens,
        &TokenPath::from_segments(["foundation", "colors", "primary"]),
    )
    .expect("primary color token should exist");

    let (source, pointer) = extract_provenance(primary_color);
    assert_eq!(source, "foundation/colors.tokens.json");
    assert_eq!(pointer, "#/foundation/colors/primary");

    let dark_background = find_token_by_path(
        &document.tokens,
        &TokenPath::from_segments(["semantic", "background", "canvas"]),
    )
    .expect("dark theme background token should exist");

    let (source, pointer) = extract_provenance(dark_background);
    assert_eq!(source, "themes/dark.tokens.json");
    assert_eq!(pointer, "#/semantic/background/canvas");
}

#[test]
fn preserves_inline_source_provenance_in_parsed_ir_extensions() {
    let resolver_json = serde_json::json!({
        "version": "2025.10",
        "sets": {
            "base": {
                "sources": [
                    {
                        "brand": {
                            "$type": "color",
                            "primary": {
                                "$value": {
                                    "colorSpace": "srgb",
                                    "components": [1.0, 0.0, 0.0],
                                    "hex": "#ff0000"
                                }
                            }
                        }
                    },
                    {
                        "brand": {
                            "$type": "color",
                            "primary": {
                                "$value": {
                                    "colorSpace": "srgb",
                                    "components": [0.0, 1.0, 0.0],
                                    "hex": "#00ff00"
                                }
                            }
                        }
                    }
                ]
            }
        },
        "resolutionOrder": [
            { "$ref": "#/sets/base" }
        ]
    });

    let resolver = Resolver::from_json(&resolver_json.to_string()).expect("resolver should load");
    let resolved = resolver
        .resolve(ResolutionInput::new())
        .expect("inline resolver should merge sources");

    let mut parser_context = ParserContext::new(resolved.to_string());
    let document = parse_document(&mut parser_context).expect("resolved JSON should parse into IR");

    assert!(
        parser_context.errors.is_empty(),
        "unexpected parse errors: {:#?}",
        parser_context.errors
    );

    let primary = find_token_by_path(
        &document.tokens,
        &TokenPath::from_segments(["brand", "primary"]),
    )
    .expect("brand primary token should exist");

    let (source, pointer) = extract_provenance(primary);
    assert_eq!(source, "$inline/1");
    assert_eq!(pointer, "#/brand/primary");
}

#[test]
fn resolves_resolver_to_ir_and_validates_graph_end_to_end() {
    let temp_dir = unique_temp_dir("tokenfoundry-resolver-e2e");
    fs::create_dir_all(&temp_dir).expect("temp directory should be created");

    let foundation_tokens = serde_json::json!({
        "foundation": {
            "colors": {
                "$type": "color",
                "primary": {
                    "$value": {
                        "colorSpace": "srgb",
                        "components": [0.2, 0.4, 1.0],
                        "hex": "#3366ff"
                    }
                },
                "secondary": {
                    "$value": "{foundation.colors.primary}"
                }
            }
        }
    });

    let semantic_tokens = serde_json::json!({
        "semantic": {
            "button": {
                "$type": "color",
                "bg": {
                    "$value": "{foundation.colors.secondary}"
                }
            }
        }
    });

    let resolver_doc = serde_json::json!({
        "version": "2025.10",
        "sets": {
            "base": {
                "sources": [
                    { "$ref": "foundation.tokens.json" },
                    { "$ref": "semantic.tokens.json" }
                ]
            }
        },
        "resolutionOrder": [
            { "$ref": "#/sets/base" }
        ]
    });

    let foundation_path = temp_dir.join("foundation.tokens.json");
    let semantic_path = temp_dir.join("semantic.tokens.json");
    let resolver_path = temp_dir.join("resolver.json");

    fs::write(&foundation_path, foundation_tokens.to_string())
        .expect("foundation tokens fixture should be written");
    fs::write(&semantic_path, semantic_tokens.to_string())
        .expect("semantic tokens fixture should be written");
    fs::write(&resolver_path, resolver_doc.to_string())
        .expect("resolver fixture should be written");

    let resolver = Resolver::from_file(&resolver_path).expect("resolver should load from file");
    let resolved = resolver
        .resolve(ResolutionInput::new())
        .expect("resolver should produce merged token json");

    // Resolver alias materialization should replace aliases with concrete values.
    assert_eq!(
        resolved["semantic"]["button"]["bg"]["$value"]["hex"],
        "#3366ff"
    );

    let mut parser_context = ParserContext::new(resolved.to_string());
    let document = parse_document(&mut parser_context)
        .expect("resolved output should parse into an IR document");

    assert!(
        parser_context.errors.is_empty(),
        "unexpected parse errors: {:#?}",
        parser_context.errors
    );

    let button_bg = find_token_by_path(
        &document.tokens,
        &TokenPath::from_segments(["semantic", "button", "bg"]),
    )
    .expect("semantic.button.bg token should exist in parsed IR");

    assert_eq!(button_bg.token_type, IrTokenType::Color);
    assert!(matches!(
        button_bg.value,
        TokenValue::Value(IrTokenValue::Color(_))
    ));

    let graph = TokenGraph::from_ir_document(&document);
    let diagnostics = graph.validate(&document);

    assert!(
        diagnostics.is_empty(),
        "unexpected graph validation diagnostics: {:#?}",
        diagnostics
    );

    // Because aliases were materialized before parsing, the graph has no Alias edges.
    assert_eq!(graph.count_edges_by_kind(EdgeKind::Alias), 0);

    let _ = fs::remove_file(&resolver_path);
    let _ = fs::remove_file(&foundation_path);
    let _ = fs::remove_file(&semantic_path);
    let _ = fs::remove_dir(&temp_dir);
}

#[test]
fn resolves_resolver_to_ir_with_alias_preservation_for_reference_outputs() {
    let temp_dir = unique_temp_dir("tokenfoundry-resolver-e2e-preserve-alias");
    fs::create_dir_all(&temp_dir).expect("temp directory should be created");

    let foundation_tokens = serde_json::json!({
        "foundation": {
            "colors": {
                "$type": "color",
                "primary": {
                    "$value": {
                        "colorSpace": "srgb",
                        "components": [0.2, 0.4, 1.0],
                        "hex": "#3366ff"
                    }
                },
                "secondary": {
                    "$value": "{foundation.colors.primary}"
                }
            }
        }
    });

    let semantic_tokens = serde_json::json!({
        "semantic": {
            "button": {
                "$type": "color",
                "bg": {
                    "$value": "{foundation.colors.secondary}"
                }
            }
        }
    });

    let resolver_doc = serde_json::json!({
        "version": "2025.10",
        "sets": {
            "base": {
                "sources": [
                    { "$ref": "foundation.tokens.json" },
                    { "$ref": "semantic.tokens.json" }
                ]
            }
        },
        "resolutionOrder": [
            { "$ref": "#/sets/base" }
        ]
    });

    let foundation_path = temp_dir.join("foundation.tokens.json");
    let semantic_path = temp_dir.join("semantic.tokens.json");
    let resolver_path = temp_dir.join("resolver.json");

    fs::write(&foundation_path, foundation_tokens.to_string())
        .expect("foundation tokens fixture should be written");
    fs::write(&semantic_path, semantic_tokens.to_string())
        .expect("semantic tokens fixture should be written");
    fs::write(&resolver_path, resolver_doc.to_string())
        .expect("resolver fixture should be written");

    let resolver = Resolver::from_file(&resolver_path).expect("resolver should load from file");
    let resolved = resolver
        .resolve_with_options(
            ResolutionInput::new(),
            ResolveOptions {
                materialize_aliases: false,
                materialize_property_refs: true,
                convert_token_refs_to_aliases: false,
            },
        )
        .expect("resolver should produce merged token json");

    assert_eq!(
        resolved["semantic"]["button"]["bg"]["$value"],
        "{foundation.colors.secondary}"
    );

    let mut parser_context = ParserContext::new(resolved.to_string());
    let document = parse_document(&mut parser_context)
        .expect("resolved output should parse into an IR document");

    assert!(
        parser_context.errors.is_empty(),
        "unexpected parse errors: {:#?}",
        parser_context.errors
    );

    let button_bg = find_token_by_path(
        &document.tokens,
        &TokenPath::from_segments(["semantic", "button", "bg"]),
    )
    .expect("semantic.button.bg token should exist in parsed IR");

    assert!(matches!(button_bg.value, TokenValue::Alias(_)));

    let graph = TokenGraph::from_ir_document(&document);
    let diagnostics = graph.validate(&document);

    assert!(
        diagnostics.is_empty(),
        "unexpected graph validation diagnostics: {:#?}",
        diagnostics
    );

    assert!(graph.count_edges_by_kind(EdgeKind::Alias) >= 1);

    let _ = fs::remove_file(&resolver_path);
    let _ = fs::remove_file(&foundation_path);
    let _ = fs::remove_file(&semantic_path);
    let _ = fs::remove_dir(&temp_dir);
}

#[test]
fn resolves_complex_resolver_file_to_graph_with_options_via_convenience_api() {
    let resolver = Resolver::from_file(complex_resolver_fixture_path())
        .expect("complex resolver fixture should load");

    let mut input = ResolutionInput::new();
    input.add_selection("theme".to_string(), "dark".to_string());

    let output = resolver
        .resolve_to_graph_with_options(
            input,
            ResolveOptions {
                materialize_aliases: false,
                materialize_property_refs: true,
                convert_token_refs_to_aliases: true,
            },
        )
        .expect("complex resolver should resolve into a graph");

    assert!(
        output.parser_diagnostics.is_empty(),
        "unexpected parse diagnostics: {:#?}",
        output.parser_diagnostics
    );
    assert!(
        output.graph_diagnostics.is_empty(),
        "unexpected graph diagnostics: {:#?}",
        output.graph_diagnostics
    );

    assert!(output.document.tokens.len() >= 2);
    assert!(output.graph.node_count() >= 8);
    assert!(output.graph.edge_count() >= 1);

    // Property refs are materialized for output emitters.
    assert_eq!(output.graph.count_edges_by_kind(EdgeKind::PropertyRef), 0);
    // Alias semantics are retained at token level.
    assert!(output.graph.count_edges_by_kind(EdgeKind::Alias) >= 1);

    assert_eq!(
        output.resolved_tokens["semantic"]["body"]["$value"]["fontFamily"],
        serde_json::json!(["Inter", "Arial", "sans-serif"])
    );
}

#[test]
fn finalized_pipeline_produces_value_or_alias_tokens_with_extends_and_root_support() {
    let resolver_json = serde_json::json!({
        "version": "2025.10",
        "sets": {
            "base": {
                "sources": [
                    {
                        "base": {
                            "button": {
                                "$type": "dimension",
                                "$root": {
                                    "$value": { "value": 8, "unit": "px" }
                                },
                                "md": {
                                    "$value": { "value": 16, "unit": "px" }
                                }
                            }
                        },
                        "theme": {
                            "$extends": "{base}",
                            "button": {
                                "lg": {
                                    "$type": "dimension",
                                    "$value": { "$ref": "#/base/button/$root/$value" }
                                }
                            }
                        }
                    }
                ]
            }
        },
        "resolutionOrder": [
            { "$ref": "#/sets/base" }
        ]
    });

    let resolver = Resolver::from_json(&resolver_json.to_string())
        .expect("resolver should parse from inline fixture");

    let output = resolve_to_finalized_pipeline(
        &resolver,
        ResolvePipelineRequest::with_defaults(ResolutionInput::new()),
    )
    .expect("finalized pipeline should succeed");

    assert!(
        output.parser_diagnostics.is_empty(),
        "unexpected parse diagnostics: {:#?}",
        output.parser_diagnostics
    );
    assert!(
        output.graph_diagnostics.is_empty(),
        "unexpected graph diagnostics: {:#?}",
        output.graph_diagnostics
    );

    let inherited_root = output
        .emittable_tokens
        .iter()
        .find(|token| token.path == TokenPath::from_segments(["theme", "button", "$root"]))
        .expect("theme.button.$root should exist via group extension");

    assert!(
        matches!(
            inherited_root.value,
            EmittableValue::Literal(IrTokenValue::Dimension(_))
        ),
        "inherited $root token should be a literal dimension"
    );

    let lg = output
        .emittable_tokens
        .iter()
        .find(|token| token.path == TokenPath::from_segments(["theme", "button", "lg"]))
        .expect("theme.button.lg should exist");

    match &lg.value {
        EmittableValue::Alias { target_path } => {
            assert_eq!(
                target_path,
                &TokenPath::from_segments(["base", "button", "$root"])
            );
        }
        other => panic!("expected alias for theme.button.lg, got {other:?}"),
    }
}
