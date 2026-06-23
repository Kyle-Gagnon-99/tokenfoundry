use tokenfoundry_core::{
    ParserContext,
    ir::{
        IrNode, IrTokenType, TokenPath, TokenValue, find_node_by_path, find_token_by_path,
        token_exists,
    },
    parsing::parse_document,
};

const FULL_SPECTRUM_FIXTURE: &str =
    include_str!("resources/tests/integration/full-spectrum.tokens.json");
const INVALID_IR_FIXTURE: &str = include_str!("resources/tests/integration/invalid-ir.tokens.json");

#[derive(Default, Debug)]
struct ParseStats {
    groups: usize,
    tokens: usize,
    literal_values: usize,
    alias_values: usize,
    ref_values: usize,
    color: usize,
    dimension: usize,
    font_family: usize,
    font_weight: usize,
    duration: usize,
    cubic_bezier: usize,
    number: usize,
    stroke_style: usize,
    border: usize,
    transition: usize,
    shadow: usize,
    gradient: usize,
    typography: usize,
}

fn collect_stats(nodes: &[IrNode], stats: &mut ParseStats) {
    for node in nodes {
        match node {
            IrNode::Group(group) => {
                stats.groups += 1;
                collect_stats(&group.children, stats);
            }
            IrNode::Token(token) => {
                stats.tokens += 1;

                match token.value {
                    TokenValue::Value(_) => stats.literal_values += 1,
                    TokenValue::Alias(_) => stats.alias_values += 1,
                    TokenValue::Ref(_) => stats.ref_values += 1,
                }

                match token.token_type {
                    IrTokenType::Color => stats.color += 1,
                    IrTokenType::Dimension => stats.dimension += 1,
                    IrTokenType::FontFamily => stats.font_family += 1,
                    IrTokenType::FontWeight => stats.font_weight += 1,
                    IrTokenType::Duration => stats.duration += 1,
                    IrTokenType::CubicBezier => stats.cubic_bezier += 1,
                    IrTokenType::Number => stats.number += 1,
                    IrTokenType::StrokeStyle => stats.stroke_style += 1,
                    IrTokenType::Border => stats.border += 1,
                    IrTokenType::Transition => stats.transition += 1,
                    IrTokenType::Shadow => stats.shadow += 1,
                    IrTokenType::Gradient => stats.gradient += 1,
                    IrTokenType::Typography => stats.typography += 1,
                }
            }
        }
    }
}

#[test]
fn parses_full_spectrum_fixture_into_ir_without_diagnostics() {
    let mut ctx = ParserContext::new(FULL_SPECTRUM_FIXTURE.to_string());

    let document = parse_document(&mut ctx).expect("fixture should parse into an IR document");

    assert!(
        ctx.errors.is_empty(),
        "unexpected parse errors: {:#?}",
        ctx.errors
    );
    assert!(
        ctx.warnings.is_empty(),
        "unexpected parse warnings: {:#?}",
        ctx.warnings
    );

    let mut stats = ParseStats::default();
    collect_stats(&document.tokens, &mut stats);

    assert_eq!(stats.groups, 16);
    assert_eq!(stats.tokens, 23);
    assert_eq!(stats.literal_values, 18);
    assert_eq!(stats.alias_values, 1);
    assert_eq!(stats.ref_values, 4);

    assert_eq!(stats.color, 4);
    assert_eq!(stats.dimension, 3);
    assert_eq!(stats.font_family, 2);
    assert_eq!(stats.font_weight, 2);
    assert_eq!(stats.duration, 2);
    assert_eq!(stats.cubic_bezier, 1);
    assert_eq!(stats.number, 2);
    assert_eq!(stats.stroke_style, 2);
    assert_eq!(stats.border, 1);
    assert_eq!(stats.transition, 1);
    assert_eq!(stats.shadow, 1);
    assert_eq!(stats.gradient, 1);
    assert_eq!(stats.typography, 1);

    println!("Parse stats: {:#?}", stats);
    println!("Parsed IR: {:#?}", document);
}

#[test]
fn reports_ir_parsing_diagnostics_for_invalid_fixture() {
    let mut ctx = ParserContext::new(INVALID_IR_FIXTURE.to_string());

    let document = parse_document(&mut ctx).expect("invalid fixture should still deserialize");

    assert!(ctx.warnings.is_empty());
    assert!(
        ctx.errors.len() >= 7,
        "expected multiple parse errors: {:#?}",
        ctx.errors
    );

    assert!(
        ctx.errors
            .iter()
            .any(|error| { error.message.contains("Missing token type") })
    );
    assert!(ctx.errors.iter().any(|error| {
        error.message.contains("Invalid group token type: paint")
            && error.path == "/invalidGroupType"
    }));
    assert!(ctx.errors.iter().any(|error| {
        error.message.contains("Invalid token type: paint")
            && error.path == "/invalidTokenType/explicitBadType"
    }));
    assert!(
        ctx.errors
            .iter()
            .any(|error| { error.path == "/badColor/missingRequiredFields/$value/colorSpace" })
    );
    assert!(
        ctx.errors
            .iter()
            .any(|error| { error.path == "/badColor/missingRequiredFields/$value/components" })
    );
    assert!(ctx.errors.iter().any(|error| {
        error.message.contains("Invalid DTCG alias format")
            && error.path == "/badColor/badAlias/$value"
    }));
    assert!(ctx.errors.iter().any(|error| {
        error
            .message
            .contains("Expected an object for dimension token value")
            && error.path == "/badDimension/wrongShape/$value"
    }));
    assert!(ctx.errors.iter().any(|error| {
        error.message.contains("Invalid JsonRefObject format")
            && error.path == "/badReference/brokenRefObject/$value"
    }));

    let mut stats = ParseStats::default();
    collect_stats(&document.tokens, &mut stats);

    assert_eq!(stats.number, 1);
    assert_eq!(stats.tokens, 1);
    assert_eq!(stats.literal_values, 1);
}

#[test]
fn finds_tokens_by_path_in_tree_structure() {
    let mut ctx = ParserContext::new(FULL_SPECTRUM_FIXTURE.to_string());

    let document = parse_document(&mut ctx).expect("fixture should parse into an IR document");

    // Test finding a token that exists - colors.brand.primary
    let path = TokenPath::from_segments(vec!["colors", "brand", "primary"]);
    assert!(
        token_exists(&document.tokens, &path),
        "token_exists should find colors.brand.primary"
    );

    if let Some(token) = find_token_by_path(&document.tokens, &path) {
        assert_eq!(token.token_type, IrTokenType::Color);
        println!(
            "✓ Found token at colors.brand.primary: {:?}",
            token.common.name
        );
    } else {
        panic!("find_token_by_path should find colors.brand.primary");
    }

    // Test finding another token - spacing.sm
    let spacing_path = TokenPath::from_segments(vec!["spacing", "sm"]);
    assert!(
        token_exists(&document.tokens, &spacing_path),
        "token_exists should find spacing.sm"
    );

    if let Some(token) = find_token_by_path(&document.tokens, &spacing_path) {
        assert_eq!(token.token_type, IrTokenType::Dimension);
        println!("✓ Found token at spacing.sm: {:?}", token.common.name);
    } else {
        panic!("find_token_by_path should find spacing.sm");
    }

    // Test finding a group node - colors.brand
    let group_path = TokenPath::from_segments(vec!["colors", "brand"]);
    if let Some(node) = find_node_by_path(&document.tokens, &group_path) {
        match node {
            IrNode::Group(_) => {
                println!("✓ Found group node at colors.brand");
            }
            IrNode::Token(_) => {
                panic!("Expected a group node at colors.brand");
            }
        }
    } else {
        panic!("find_node_by_path should find colors.brand");
    }

    // Test that non-existent path returns None
    let non_existent_path = TokenPath::from_segments(vec!["nonexistent", "path", "token"]);
    assert!(
        !token_exists(&document.tokens, &non_existent_path),
        "token_exists should return false for non-existent path"
    );
    assert!(
        find_token_by_path(&document.tokens, &non_existent_path).is_none(),
        "find_token_by_path should return None for non-existent path"
    );

    println!("✓ All tree traversal tests passed!");
}
