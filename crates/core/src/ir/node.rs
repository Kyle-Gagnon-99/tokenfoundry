//! The `node` module contains the definitions for what a node is in the IR and the parsed DTCG format.

use crate::{
    ir::{JsonRef, ParseState, TokenAlias, TokenCommon},
    token::{
        BorderTokenValue, ColorTokenValue, CubicBezierTokenValue, DimensionTokenValue,
        DurationTokenValue, FontFamilyTokenValue, FontWeightTokenValue, GradientTokenValue,
        NumberTokenValue, ShadowTokenValue, StrokeStyleTokenValue, TransitionTokenValue,
        TypographyTokenValue,
    },
};

/// The `TokenValue` enum represents the source of a token's value in the IR, which can either be a literal value of type `T`,
/// an alias to another token or a reference to another token.
#[derive(Debug, Clone, PartialEq)]
pub enum TokenValue {
    Value(IrTokenValue),
    Alias(TokenAlias),
    Ref(JsonRef),
}

/// The `IrTokenType` enum represents the different tokens.
#[derive(Debug, Clone, PartialEq)]
pub enum IrTokenValue {
    Color(ColorTokenValue),
    Dimension(DimensionTokenValue),
    FontFamily(FontFamilyTokenValue),
    FontWeight(FontWeightTokenValue),
    Duration(DurationTokenValue),
    CubicBezier(CubicBezierTokenValue),
    Number(NumberTokenValue),
    StrokeStyle(StrokeStyleTokenValue),
    Border(BorderTokenValue),
    Transition(TransitionTokenValue),
    Shadow(ShadowTokenValue),
    Gradient(GradientTokenValue),
    Typography(TypographyTokenValue),
}

impl From<ParseState<ColorTokenValue>> for ParseState<IrTokenValue> {
    fn from(value: ParseState<ColorTokenValue>) -> Self {
        match value {
            ParseState::Parsed(v) => ParseState::Parsed(IrTokenValue::Color(v)),
            ParseState::Invalid(reason) => ParseState::Invalid(reason),
            ParseState::NoMatch => ParseState::NoMatch,
        }
    }
}

impl From<ParseState<DimensionTokenValue>> for ParseState<IrTokenValue> {
    fn from(value: ParseState<DimensionTokenValue>) -> Self {
        match value {
            ParseState::Parsed(v) => ParseState::Parsed(IrTokenValue::Dimension(v)),
            ParseState::Invalid(reason) => ParseState::Invalid(reason),
            ParseState::NoMatch => ParseState::NoMatch,
        }
    }
}

impl From<ParseState<FontFamilyTokenValue>> for ParseState<IrTokenValue> {
    fn from(value: ParseState<FontFamilyTokenValue>) -> Self {
        match value {
            ParseState::Parsed(v) => ParseState::Parsed(IrTokenValue::FontFamily(v)),
            ParseState::Invalid(reason) => ParseState::Invalid(reason),
            ParseState::NoMatch => ParseState::NoMatch,
        }
    }
}

impl From<ParseState<FontWeightTokenValue>> for ParseState<IrTokenValue> {
    fn from(value: ParseState<FontWeightTokenValue>) -> Self {
        match value {
            ParseState::Parsed(v) => ParseState::Parsed(IrTokenValue::FontWeight(v)),
            ParseState::Invalid(reason) => ParseState::Invalid(reason),
            ParseState::NoMatch => ParseState::NoMatch,
        }
    }
}

impl From<ParseState<DurationTokenValue>> for ParseState<IrTokenValue> {
    fn from(value: ParseState<DurationTokenValue>) -> Self {
        match value {
            ParseState::Parsed(v) => ParseState::Parsed(IrTokenValue::Duration(v)),
            ParseState::Invalid(reason) => ParseState::Invalid(reason),
            ParseState::NoMatch => ParseState::NoMatch,
        }
    }
}

impl From<ParseState<CubicBezierTokenValue>> for ParseState<IrTokenValue> {
    fn from(value: ParseState<CubicBezierTokenValue>) -> Self {
        match value {
            ParseState::Parsed(v) => ParseState::Parsed(IrTokenValue::CubicBezier(v)),
            ParseState::Invalid(reason) => ParseState::Invalid(reason),
            ParseState::NoMatch => ParseState::NoMatch,
        }
    }
}

impl From<ParseState<NumberTokenValue>> for ParseState<IrTokenValue> {
    fn from(value: ParseState<NumberTokenValue>) -> Self {
        match value {
            ParseState::Parsed(v) => ParseState::Parsed(IrTokenValue::Number(v)),
            ParseState::Invalid(reason) => ParseState::Invalid(reason),
            ParseState::NoMatch => ParseState::NoMatch,
        }
    }
}

impl From<ParseState<StrokeStyleTokenValue>> for ParseState<IrTokenValue> {
    fn from(value: ParseState<StrokeStyleTokenValue>) -> Self {
        match value {
            ParseState::Parsed(v) => ParseState::Parsed(IrTokenValue::StrokeStyle(v)),
            ParseState::Invalid(reason) => ParseState::Invalid(reason),
            ParseState::NoMatch => ParseState::NoMatch,
        }
    }
}

impl From<ParseState<BorderTokenValue>> for ParseState<IrTokenValue> {
    fn from(value: ParseState<BorderTokenValue>) -> Self {
        match value {
            ParseState::Parsed(v) => ParseState::Parsed(IrTokenValue::Border(v)),
            ParseState::Invalid(reason) => ParseState::Invalid(reason),
            ParseState::NoMatch => ParseState::NoMatch,
        }
    }
}

impl From<ParseState<TransitionTokenValue>> for ParseState<IrTokenValue> {
    fn from(value: ParseState<TransitionTokenValue>) -> Self {
        match value {
            ParseState::Parsed(v) => ParseState::Parsed(IrTokenValue::Transition(v)),
            ParseState::Invalid(reason) => ParseState::Invalid(reason),
            ParseState::NoMatch => ParseState::NoMatch,
        }
    }
}

impl From<ParseState<ShadowTokenValue>> for ParseState<IrTokenValue> {
    fn from(value: ParseState<ShadowTokenValue>) -> Self {
        match value {
            ParseState::Parsed(v) => ParseState::Parsed(IrTokenValue::Shadow(v)),
            ParseState::Invalid(reason) => ParseState::Invalid(reason),
            ParseState::NoMatch => ParseState::NoMatch,
        }
    }
}

impl From<ParseState<GradientTokenValue>> for ParseState<IrTokenValue> {
    fn from(value: ParseState<GradientTokenValue>) -> Self {
        match value {
            ParseState::Parsed(v) => ParseState::Parsed(IrTokenValue::Gradient(v)),
            ParseState::Invalid(reason) => ParseState::Invalid(reason),
            ParseState::NoMatch => ParseState::NoMatch,
        }
    }
}

impl From<ParseState<TypographyTokenValue>> for ParseState<IrTokenValue> {
    fn from(value: ParseState<TypographyTokenValue>) -> Self {
        match value {
            ParseState::Parsed(v) => ParseState::Parsed(IrTokenValue::Typography(v)),
            ParseState::Invalid(reason) => ParseState::Invalid(reason),
            ParseState::NoMatch => ParseState::NoMatch,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IrTokenType {
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

impl IrTokenType {
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "color" => Some(IrTokenType::Color),
            "dimension" => Some(IrTokenType::Dimension),
            "fontFamily" => Some(IrTokenType::FontFamily),
            "fontWeight" => Some(IrTokenType::FontWeight),
            "duration" => Some(IrTokenType::Duration),
            "cubicBezier" => Some(IrTokenType::CubicBezier),
            "number" => Some(IrTokenType::Number),
            "strokeStyle" => Some(IrTokenType::StrokeStyle),
            "border" => Some(IrTokenType::Border),
            "transition" => Some(IrTokenType::Transition),
            "shadow" => Some(IrTokenType::Shadow),
            "gradient" => Some(IrTokenType::Gradient),
            "typography" => Some(IrTokenType::Typography),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct IrToken {
    pub common: TokenCommon,
    pub token_type: IrTokenType,
    pub value: TokenValue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IrGroupToken {
    pub common: TokenCommon,
    pub children: Vec<IrNode>,
}

/// Parsed token tree for a single parser invocation.
///
/// The parser may consume resolver-merged content, so this is intentionally
/// identity-free at the document level. Source attribution is carried by
/// per-token provenance in `TokenCommon`.
#[derive(Debug, Clone, PartialEq)]
pub struct IrDocument {
    pub tokens: Vec<IrNode>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IrNode {
    Token(IrToken),
    Group(IrGroupToken),
}
