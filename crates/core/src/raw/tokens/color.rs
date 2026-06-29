//! The `color` module contains the raw serde representation of DTCG color tokens, which are used for deserialization of token data.

use ordered_float::OrderedFloat;
use serde::{Deserialize, Deserializer, de::Visitor};

use crate::raw::RefOrLiteral;

/// Represents a color component element in the color component array of a DTCG color token.
/// The color component can either be a number or a "none" string value.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum RawColorComponent {
    Number(OrderedFloat<f64>),
    None,
}

impl<'de> Deserialize<'de> for RawColorComponent {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct RawColorComponentVisitor;

        impl<'de> Visitor<'de> for RawColorComponentVisitor {
            type Value = RawColorComponent;

            fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                formatter.write_str("a number or the string 'none'")
            }

            fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(RawColorComponent::Number(OrderedFloat(v)))
            }

            fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(RawColorComponent::Number(OrderedFloat(v as f64)))
            }

            fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                Ok(RawColorComponent::Number(OrderedFloat(v as f64)))
            }

            fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
            where
                E: serde::de::Error,
            {
                if v.to_lowercase() == "none" {
                    Ok(RawColorComponent::None)
                } else {
                    Err(serde::de::Error::custom(format!(
                        "expected 'none' or a number, got '{}'",
                        v
                    )))
                }
            }
        }

        deserializer.deserialize_any(RawColorComponentVisitor)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
pub enum RawColorSpace {
    #[serde(rename = "srgb")]
    SRGB,
    #[serde(rename = "srgb-linear")]
    SRGBLinear,
    #[serde(rename = "hsl")]
    HSL,
    #[serde(rename = "hwb")]
    HWB,
    #[serde(rename = "lab")]
    CIELAB,
    #[serde(rename = "lch")]
    LCH,
    #[serde(rename = "oklab")]
    OKLAB,
    #[serde(rename = "oklch")]
    OKLCH,
    #[serde(rename = "display-p3")]
    DisplayP3,
    #[serde(rename = "a98-rgb")]
    A98RGB,
    #[serde(rename = "prophoto-rgb")]
    ProPhotoRGB,
    #[serde(rename = "rec2020")]
    Rec2020,
    #[serde(rename = "xyz-d50")]
    XYZD50,
    #[serde(rename = "xyz-d65")]
    XYZD65,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RawColor {
    pub color_space: RawColorSpace,
    pub components: RefOrLiteral<Vec<RefOrLiteral<RawColorComponent>>>,
    pub alpha: Option<RefOrLiteral<OrderedFloat<f64>>>,
    pub hex: Option<RefOrLiteral<String>>,
}
