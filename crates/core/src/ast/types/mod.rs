//! The `types` module provides the AST representation of all of the types defined in DTCG.

use crate::{
    ast::{
        node::PropId,
        types::{
            border::Border, color::Color, cubic_bezier::CubicBezier, dimension::Dimension,
            duration::Duration, font_family::FontFamily, font_weight::FontWeight,
            gradient::Gradient, number::Number, shadow::Shadow, stroke_style::StrokeStyle,
            transition::Transition, typography::Typography,
        },
    },
    diagnostics::diagnostic::DiagnosticSink,
    ir::pipeline::Analyzed,
};

pub mod border;
pub mod color;
pub mod cubic_bezier;
pub mod dimension;
pub mod duration;
pub mod font_family;
pub mod font_weight;
pub mod gradient;
pub mod number;
pub mod shadow;
pub mod stroke_style;
pub mod transition;
pub mod typography;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum TokenType {
    Color,
    Dimension,
    FontFamily,
    FontWeight,
    Duration,
    CubicBezier,
    Number,
    StrokeStyle,
    Border,
    Transition,
    Shadow,
    Gradient,
    Typography,
}

impl TokenType {
    /// Converts a string representation of a token type into the corresponding `TokenType` enum variant.
    /// Returns `None` if the string does not match any known token type.
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "color" => Some(TokenType::Color),
            "dimension" => Some(TokenType::Dimension),
            "fontFamily" => Some(TokenType::FontFamily),
            "fontWeight" => Some(TokenType::FontWeight),
            "duration" => Some(TokenType::Duration),
            "cubicBezier" => Some(TokenType::CubicBezier),
            "number" => Some(TokenType::Number),
            "strokeStyle" => Some(TokenType::StrokeStyle),
            "border" => Some(TokenType::Border),
            "transition" => Some(TokenType::Transition),
            "shadow" => Some(TokenType::Shadow),
            "gradient" => Some(TokenType::Gradient),
            "typography" => Some(TokenType::Typography),
            _ => None,
        }
    }
}

impl TokenType {
    pub fn as_str(self) -> &'static str {
        match self {
            TokenType::Color => "color",
            TokenType::Dimension => "dimension",
            TokenType::FontFamily => "fontFamily",
            TokenType::FontWeight => "fontWeight",
            TokenType::Duration => "duration",
            TokenType::CubicBezier => "cubicBezier",
            TokenType::Number => "number",
            TokenType::StrokeStyle => "strokeStyle",
            TokenType::Border => "border",
            TokenType::Transition => "transition",
            TokenType::Shadow => "shadow",
            TokenType::Gradient => "gradient",
            TokenType::Typography => "typography",
        }
    }
}

/// The parsed `$value` of a token, one variant per DTCG type.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenValue {
    Color(Color),
    Dimension(Dimension),
    Number(Number),
    Duration(Duration),
    CubicBezier(CubicBezier),
    Shadow(Shadow),
    FontFamily(FontFamily),
    FontWeight(FontWeight),
    StrokeStyle(StrokeStyle),
    Border(Border),
    Transition(Transition),
    Gradient(Gradient),
    Typography(Typography),
}

/// Parses a value shape. Implementations report every problem they find and return `None` when no usable value
/// could be built.
pub trait FromProp: Sized {
    fn from_prop(prop: PropId, context: &Analyzed, diags: &mut impl DiagnosticSink)
    -> Option<Self>;
}

/// Parses the literal `$value` at `prop` as the given type.
pub fn parse_token_value(
    ty: TokenType,
    prop: PropId,
    a: &Analyzed,
    diags: &mut impl DiagnosticSink,
) -> Option<TokenValue> {
    match ty {
        TokenType::Color => Color::from_prop(prop, a, diags).map(TokenValue::Color),
        TokenType::Dimension => Dimension::from_prop(prop, a, diags).map(TokenValue::Dimension),
        TokenType::Number => Number::from_prop(prop, a, diags).map(TokenValue::Number),
        TokenType::Duration => Duration::from_prop(prop, a, diags).map(TokenValue::Duration),
        TokenType::CubicBezier => {
            CubicBezier::from_prop(prop, a, diags).map(TokenValue::CubicBezier)
        }
        TokenType::Shadow => Shadow::from_prop(prop, a, diags).map(TokenValue::Shadow),
        TokenType::FontFamily => FontFamily::from_prop(prop, a, diags).map(TokenValue::FontFamily),
        TokenType::FontWeight => FontWeight::from_prop(prop, a, diags).map(TokenValue::FontWeight),
        TokenType::StrokeStyle => {
            StrokeStyle::from_prop(prop, a, diags).map(TokenValue::StrokeStyle)
        }
        TokenType::Border => Border::from_prop(prop, a, diags).map(TokenValue::Border),
        TokenType::Transition => Transition::from_prop(prop, a, diags).map(TokenValue::Transition),
        TokenType::Gradient => Gradient::from_prop(prop, a, diags).map(TokenValue::Gradient),
        TokenType::Typography => Typography::from_prop(prop, a, diags).map(TokenValue::Typography),
    }
}

/// Helpers the type modules share to test their parsers.
#[cfg(test)]
pub(crate) mod testing {
    use super::*;
    use crate::{
        diagnostics::{code::DiagnosticCode, diagnostic::DiagnosticStore},
        ir::{
            lower::{PropKind, TokenBody},
            merge::LayerOrigin,
            pipeline::{Layer, analyze},
        },
        source::SourceIdGenerator,
    };

    /// Analyzes `{"t": {"$type": <ty>, "$value": <value>}}` plus `extra` top-level members, and parses the `$value`
    /// as a `T`.
    pub fn parse<T: FromProp>(
        ty: &str,
        value: &str,
        extra: &str,
    ) -> (Option<T>, DiagnosticStore, Analyzed) {
        let doc = format!(r#"{{"t": {{"$type": "{ty}", "$value": {value}}}{extra}}}"#);
        let (tree, errors) = json_spanned::parse_json(&doc);
        assert!(errors.is_empty(), "bad test json: {doc}");
        let tree = tree.unwrap();

        let source = SourceIdGenerator::default().generate();
        let layers = [Layer {
            source,
            origin: LayerOrigin::File(source),
            tree: &tree,
        }];
        let analyzed = analyze(&layers, &mut DiagnosticStore::new()).expect("no cycles");

        let token = analyzed.lowered.member(&["t"]).unwrap();
        let PropKind::Token(data) = &analyzed.lowered.get(token).kind else {
            panic!()
        };
        let TokenBody::Value(v) = data.body else {
            panic!("no $value")
        };

        let mut diags = DiagnosticStore::new();
        let result = T::from_prop(v, &analyzed, &mut diags);
        (result, diags, analyzed)
    }

    pub fn codes(d: &DiagnosticStore) -> Vec<DiagnosticCode> {
        d.diagnostics().iter().map(|d| d.code).collect()
    }

    /// Parses and expects no diagnostics at all.
    pub fn ok<T: FromProp>(ty: &str, value: &str, extra: &str) -> T {
        let (result, diags, _) = parse::<T>(ty, value, extra);
        assert!(diags.diagnostics().is_empty(), "{:?}", diags.diagnostics());
        result.expect("a value")
    }
}
