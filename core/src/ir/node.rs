//! The `node` module contains the definitions for what a node is in the IR and the parsed DTCG format.

use crate::{
    ParserContext,
    ir::{DocumentId, JsonObject, JsonRef, TokenAlias, TokenCommon, TryFromJson},
    token::token_types::{
        BorderTokenValue, ShadowTokenValue, StrokeStyleTokenValue, TransitionTokenValue,
        TypographyTokenValue, color::ColorTokenValue, cubic_bezier::CubicBezierTokenValue,
        dimension::DimensionTokenValue, duration::DurationTokenValue,
        font_family::FontFamilyTokenValue, font_weight::FontWeightTokenValue,
        number::NumberTokenValue,
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

impl<'a> TokenValue {
    pub fn parse_token_value(
        ctx: &mut ParserContext,
        path: &str,
        token_type: IrTokenType,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        match value {
            serde_json::Value::String(str_val) => {
                if let Some(alias) = TokenAlias::from_dtcg_alias(str_val) {
                    Some(TokenValue::Alias(alias))
                } else {
                    IrTokenValue::parse_token(ctx, path, token_type, value).map(TokenValue::Value)
                }
            }
            serde_json::Value::Object(map) => {
                let json_obj = JsonObject::new(map);
                if json_obj.is_ref_object() {
                    match json_obj.get_ref(ctx, path) {
                        Some(json_ref) => Some(TokenValue::Ref(json_ref)),
                        None => None, // The error has already been pushed by get_ref, so we just return None here
                    }
                } else {
                    IrTokenValue::parse_token(ctx, path, token_type, value).map(TokenValue::Value)
                }
            }
            _ => IrTokenValue::parse_token(ctx, path, token_type, value).map(TokenValue::Value),
        }
    }
}

/// The `IrTokenType` enum represents the different tokens.
#[derive(Debug, Clone, PartialEq)]
pub enum IrTokenValue {
    Dimension(DimensionTokenValue),
    Color(ColorTokenValue),
    CubicBezier(CubicBezierTokenValue),
    Duration(DurationTokenValue),
    FontFamily(FontFamilyTokenValue),
    FontWeight(FontWeightTokenValue),
    Number(NumberTokenValue),
    Border(BorderTokenValue),
    Shadow(ShadowTokenValue),
    StrokeStyle(StrokeStyleTokenValue),
    Transition(TransitionTokenValue),
    Typography(TypographyTokenValue),
}

#[derive(Debug, Clone, PartialEq)]
pub enum IrTokenType {
    Dimension,
    Color,
    CubicBezier,
    Duration,
    FontFamily,
    FontWeight,
    Number,
    Border,
    Shadow,
    StrokeStyle,
    Transition,
    Typography,
}

impl<'a> IrTokenValue {
    pub fn parse_token(
        ctx: &mut ParserContext,
        path: &str,
        token_type: IrTokenType,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        match token_type {
            IrTokenType::Dimension => {
                DimensionTokenValue::try_from_json(ctx, path, value).map(IrTokenValue::Dimension)
            }
            IrTokenType::Color => {
                ColorTokenValue::try_from_json(ctx, path, value).map(IrTokenValue::Color)
            }
            IrTokenType::CubicBezier => CubicBezierTokenValue::try_from_json(ctx, path, value)
                .map(IrTokenValue::CubicBezier),
            IrTokenType::Duration => {
                DurationTokenValue::try_from_json(ctx, path, value).map(IrTokenValue::Duration)
            }
            IrTokenType::FontFamily => {
                FontFamilyTokenValue::try_from_json(ctx, path, value).map(IrTokenValue::FontFamily)
            }
            IrTokenType::FontWeight => {
                FontWeightTokenValue::try_from_json(ctx, path, value).map(IrTokenValue::FontWeight)
            }
            IrTokenType::Number => {
                NumberTokenValue::try_from_json(ctx, path, value).map(IrTokenValue::Number)
            }
            IrTokenType::Border => {
                BorderTokenValue::try_from_json(ctx, path, value).map(IrTokenValue::Border)
            }
            IrTokenType::Shadow => {
                ShadowTokenValue::try_from_json(ctx, path, value).map(IrTokenValue::Shadow)
            }
            IrTokenType::StrokeStyle => StrokeStyleTokenValue::try_from_json(ctx, path, value)
                .map(IrTokenValue::StrokeStyle),
            IrTokenType::Transition => {
                TransitionTokenValue::try_from_json(ctx, path, value).map(IrTokenValue::Transition)
            }
            IrTokenType::Typography => {
                TypographyTokenValue::try_from_json(ctx, path, value).map(IrTokenValue::Typography)
            }
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
pub struct IrGroupToken<'a> {
    pub common: TokenCommon,
    pub children: Vec<&'a IrNode<'a>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IrDocument<'a> {
    pub id: DocumentId,
    pub tokens: Vec<IrNode<'a>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum IrNode<'a> {
    Token(IrToken),
    Group(IrGroupToken<'a>),
}
