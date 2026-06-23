//! The `token_types` module defines the core data structures of the token values.
//! See the (DTCG specification)[https://www.designtokens.org/tr/2025.10/format/#types] for more details on the types of tokens

pub mod color;
pub mod composite;
pub mod cubic_bezier;
pub mod dimension;
pub mod duration;
pub mod font_family;
pub mod font_weight;
pub mod number;

pub use color::ColorTokenValue;
pub use composite::*;
pub use cubic_bezier::CubicBezierTokenValue;
pub use dimension::DimensionTokenValue;
pub use duration::DurationTokenValue;
pub use font_family::FontFamilyTokenValue;
pub use font_weight::FontWeightTokenValue;
pub use number::NumberTokenValue;
