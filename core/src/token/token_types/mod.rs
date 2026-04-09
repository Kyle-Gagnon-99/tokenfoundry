//! The `token_types` module defines the core data structures of the token values.
//! See the (DTCG specification)[https://www.designtokens.org/tr/2025.10/format/#types] for more details on the types of tokens

pub mod color;
pub mod cubic_bezier;
pub mod dimension;
pub mod duration;
pub mod font_family;
pub mod font_weight;

pub enum TokenValue {
    Dimension(dimension::DimensionTokenValue),
    Color(color::ColorTokenValue),
    FontFamily(font_family::FontFamilyTokenValue),
    FontWeight(font_weight::FontWeightTokenValue),
    CubicBezier(cubic_bezier::CubicBezierTokenValue),
    Duration(duration::DurationTokenValue),
}

pub enum TokenType {
    Dimension,
    Color,
    FontFamily,
    FontWeight,
    CubicBezier,
    Duration,
}
