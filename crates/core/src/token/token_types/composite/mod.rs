//! The `composite` module defines the various composite token types in DTCG.

pub mod border;
pub mod gradient;
pub mod shadow;
pub mod stroke_style;
pub mod transition;
pub mod typography;

pub use border::BorderTokenValue;
pub use gradient::GradientTokenValue;
pub use shadow::ShadowTokenValue;
pub use stroke_style::StrokeStyleTokenValue;
pub use transition::TransitionTokenValue;
pub use typography::TypographyTokenValue;
