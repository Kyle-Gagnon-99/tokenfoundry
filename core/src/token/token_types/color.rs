//! The `color` module defines the data structures for color tokens defined in the DTCG specification

use std::f64;

use crate::{
    ParserContext,
    errors::DiagnosticCode,
    ir::{
        JsonNumber, RefOr, TryFromJson, parse_ref_or_value, require_enum_string_with_mapping,
        require_object,
    },
};

pub type HexColorTokenValue = String;

const COLOR_COMPONENT_ARRAY_LENGTH: usize = 3;

/// The `validation` module contains functions for validating color component values according to the DTCG specification.
/// These functions are typically used by the `validate` method in `ValidateColorComponentValue` implementations.
/// This module is separate from the main `color` module to keep the validation logic organized and separate from the data structures for color tokens.
/// This module is also private to the `color` module, as the validation functions are only intended to be used by the `validate` method in `ValidateColorComponentValue` implementations and should not be exposed outside of the `color` module.
mod validation {
    use crate::errors::DiagnosticCode;

    use super::*;

    /// Pushes an error to the parser context indicating that a color component value is invalid.
    ///
    /// This function is used by the `validate_color_component_value` method of the color component structs to push an error to the parser context when a color component value is invalid.
    ///
    /// # Arguments
    ///
    /// * `ctx` - The parser context to push the error to
    /// * `path` - The path to the color token in the token file (e.g. "tokens.colors.primary")
    /// * `component_path` - The path to the color component in the color token (e.g. "red" for an sRGB color token)
    /// * `message` - A message describing why the color component value is invalid
    pub fn push_invalid_color_component_value_error(
        ctx: &mut ParserContext,
        path: &str,
        component_path: &str,
        message: impl Into<String>,
    ) {
        ctx.push_to_errors(
            DiagnosticCode::InvalidTokenValue,
            format!(
                "Invalid color component value at path '{}.{}': {}",
                path,
                component_path,
                message.into()
            ),
            path.into(),
        );
    }

    /// Validates that a hex color token value is a valid hex color string according to the DTCG specification.
    /// A valid hex color token value must start with a "#" character, followed by either 3, 4, or 6 hexadecimal digits
    /// 3 digits: #RGB (e.g. #f09)
    /// 4 digits: #RGBA (e.g. #f09c)
    /// 6 digits: #RRGGBB (e.g. #ff0099)
    /// Alpha is separated by the alpha key in the color token, so it is not included in the hex value of the color token
    ///
    /// # Arguments
    ///
    /// * `value` - The hex color token value to validate
    ///
    /// # Returns
    ///
    /// * `true` if the hex color token value is valid, `false` otherwise
    pub fn validate_hex_color_token_value(value: &str) -> bool {
        let Some(hex) = value.strip_prefix('#') else {
            return false;
        };

        matches!(hex.len(), 3 | 4 | 6) && hex.chars().all(|c| c.is_ascii_hexdigit())
    }

    /// Given a color component value, validates that it is within the specified range according to the DTCG specification.
    /// If the color component value is "none", it is considered valid regardless of the range
    ///
    /// # Arguments
    ///
    /// - `ctx` - The parser context to push errors to if the color component value is invalid
    /// - `token_path` - The path to the color token in the token file (e.g. "tokens.colors.primary")
    /// - `component_name` - The name of the color component being validated (e.g. "red" for an sRGB color token)
    /// - `component` - The color component value to validate
    /// - `min` - The minimum valid value for the color component (inclusive)
    /// - `max` - The maximum valid value for the color component (inclusive if `max_inclusion` is true, exclusive if `max_inclusion` is false)
    /// - `max_inclusion` - Whether the maximum valid value for the color component is inclusive or exclusive
    ///
    /// # Returns
    ///
    /// - `true` if the color component value is valid, `false` otherwise
    pub fn validate_range(
        ctx: &mut ParserContext,
        token_path: &str,
        component_name: &str,
        component: &ColorComponentElement<f64>,
        min: f64,
        max: f64,
        max_inclusion: bool,
    ) -> bool {
        match component {
            ColorComponentElement::None => true,
            ColorComponentElement::Value(value) => {
                let deref_value = *value;
                if deref_value < min
                    || (deref_value > max || (!max_inclusion && deref_value == max))
                {
                    push_invalid_color_component_value_error(
                        ctx,
                        token_path,
                        component_name,
                        format!(
                            "Value must be between {} and {}{}",
                            min,
                            max,
                            if max_inclusion {
                                " (inclusive)"
                            } else {
                                " (exclusive)"
                            }
                        ),
                    );
                    false
                } else {
                    true
                }
            }
        }
    }

    /// Validates that a color component value is non-negative according to the DTCG specification.
    /// If the color component value is "none", it is considered valid regardless of whether it is non-negative
    ///
    /// # Arguments
    ///
    /// - `ctx` - The parser context to push errors to if the color component value is invalid
    /// - `token_path` - The path to the color token in the token file (e.g. "tokens.colors.primary")
    /// - `component_name` - The name of the color component being validated (e.g. "chroma" for an LCH color token)
    /// - `component` - The color component value to validate
    ///
    /// # Returns
    ///
    /// - `true` if the color component value is valid, `false` otherwise
    pub fn validate_non_negative(
        ctx: &mut ParserContext,
        token_path: &str,
        component_name: &str,
        component: &ColorComponentElement<f64>,
    ) -> bool {
        match component {
            ColorComponentElement::None => true,
            ColorComponentElement::Value(value) => {
                let deref_value = *value;
                if deref_value < 0.0 {
                    push_invalid_color_component_value_error(
                        ctx,
                        token_path,
                        component_name,
                        "Value must be non-negative".to_string(),
                    );
                    false
                } else {
                    true
                }
            }
        }
    }

    /// Validates that an alpha value is between 0 and 1 (inclusive) according to the DTCG specification.
    /// If the alpha value is not specified, it is considered valid (as it will be treated as 1, which is a valid alpha value)
    ///
    /// # Arguments
    ///
    /// - `ctx` - The parser context to push errors to if the alpha value is invalid
    /// - `token_path` - The path to the color token in the token file (e.g. "tokens.colors.primary")
    /// - `alpha` - The alpha value to validate
    ///
    /// # Returns
    ///
    /// - `true` if the alpha value is valid, `false` otherwise
    pub fn validate_alpha(ctx: &mut ParserContext, token_path: &str, alpha: Option<f64>) -> bool {
        if let Some(alpha_value) = alpha {
            if alpha_value < 0.0 || alpha_value > 1.0 {
                push_invalid_color_component_value_error(
                    ctx,
                    token_path,
                    "alpha",
                    "Alpha value must be between 0 and 1 (inclusive)".to_string(),
                );
                return false;
            }
        }
        true
    }

    /// Validates the components of the color tokens
    ///
    /// Validates that the length of the array is 3, and that each component is a valid number or the "none" keyword string according to the DTCG specification.
    pub fn validate_component_array(
        ctx: &mut ParserContext,
        token_path: &str,
        components: &[serde_json::Value],
    ) -> Option<[ColorComponentElement<f64>; 3]> {
        // First, check if the length of the components array is 3
        if components.len() != COLOR_COMPONENT_ARRAY_LENGTH {
            ctx.push_to_errors(DiagnosticCode::InvalidTokenValue, format!(
                "Expected an array of length {} for color components, but got an array of length {}",
                COLOR_COMPONENT_ARRAY_LENGTH,
                components.len()
            ), token_path.into());
            return None;
        }

        // Next, for each element in the array, check if it is a JSON string of "none" or a JSON number
        // Push an error to the parser context if any of the components are invalid, and return None if any of the components are invalid
        let mut parsed_components: [ColorComponentElement<f64>; COLOR_COMPONENT_ARRAY_LENGTH] = [
            ColorComponentElement::None,
            ColorComponentElement::None,
            ColorComponentElement::None,
        ];
        for (index, component) in components.iter().enumerate() {
            if let Some(string_value) = component.as_str() {
                if string_value == "none" {
                    parsed_components[index] = ColorComponentElement::None;
                } else {
                    ctx.push_to_errors(DiagnosticCode::InvalidTokenValue, format!(
                        "Expected a string with the value 'none' or a number for color components, but got a string with the value '{}'",
                        string_value
                    ), token_path.into());
                    return None;
                }
            } else if let Some(number_value) = component.as_f64() {
                parsed_components[index] = ColorComponentElement::Value(number_value);
            } else {
                ctx.push_to_errors(DiagnosticCode::InvalidTokenValue, format!(
                    "Expected a string with the value 'none' or a number for color components, but got a value of type {}",
                    component
                ), token_path.into());
                return None;
            }
        }
        Some(parsed_components)
    }
}

macro_rules! define_three_component_color_space {
    ($name:ident, $c1:ident, $c2:ident, $c3:ident) => {
        #[derive(Debug, Clone, PartialEq)]
        pub struct $name {
            $c1: RefOr<ColorComponentElement<f64>>,
            $c2: RefOr<ColorComponentElement<f64>>,
            $c3: RefOr<ColorComponentElement<f64>>,
        }

        impl $name {
            pub fn new(
                $c1: RefOr<ColorComponentElement<f64>>,
                $c2: RefOr<ColorComponentElement<f64>>,
                $c3: RefOr<ColorComponentElement<f64>>,
            ) -> Self {
                Self { $c1, $c2, $c3 }
            }
        }

        impl From<[RefOr<ColorComponentElement<f64>>; 3]> for $name {
            fn from(components: [RefOr<ColorComponentElement<f64>>; 3]) -> Self {
                Self {
                    $c1: components[0].clone(),
                    $c2: components[1].clone(),
                    $c3: components[2].clone(),
                }
            }
        }
    };
}

/// The DTCG specification defines that a color token's components can either be a "none" keyword string,
/// or a number
#[derive(Debug, Clone, PartialEq)]
pub enum ColorComponentElement<T> {
    Value(T),
    None,
}

impl<'a> TryFromJson<'a> for ColorComponentElement<f64> {
    fn try_from_json(
        ctx: &mut ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        match value {
            serde_json::Value::String(str_val) => {
                if str_val.to_lowercase() == "none" {
                    Some(ColorComponentElement::None)
                } else {
                    ctx.push_to_errors(DiagnosticCode::InvalidTokenValue, format!(
                        "Expected a string with the value 'none' or a number for color components, but got a string with the value '{}'",
                        str_val
                    ), path.into());
                    None
                }
            }
            serde_json::Value::Number(num_val) => {
                if let Some(f64_val) = num_val.as_f64() {
                    Some(ColorComponentElement::Value(f64_val))
                } else {
                    ctx.push_to_errors(DiagnosticCode::InvalidTokenValue, format!(
                        "Expected a string with the value 'none' or a number for color components, but got a number that cannot be represented as f64: {}",
                        num_val
                    ), path.into());
                    None
                }
            }
            _ => {
                ctx.push_to_errors(DiagnosticCode::InvalidTokenValue, format!(
                    "Expected a string with the value 'none' or a number for color components, but got a value of type {}",
                    value
                ), path.into());
                None
            }
        }
    }
}

/// The DTCG specification defines the valid color spaces that can be used in color tokens.
#[derive(Debug, Clone, PartialEq)]
pub enum ColorSpace {
    SRGB,
    SRGBLinear,
    HSL,
    HWB,
    CIELAB,
    LCH,
    OKLAB,
    OKLCH,
    DisplayP3,
    A98RGB,
    ProPhotoRGB,
    Rec2020,
    XYZD65,
    XYZD50,
}

impl<'a> TryFromJson<'a> for ColorSpace {
    fn try_from_json(
        ctx: &mut ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        let mapping = [
            ("srgb", ColorSpace::SRGB),
            ("srgb-linear", ColorSpace::SRGBLinear),
            ("hsl", ColorSpace::HSL),
            ("hwb", ColorSpace::HWB),
            ("cielab", ColorSpace::CIELAB),
            ("lch", ColorSpace::LCH),
            ("oklab", ColorSpace::OKLAB),
            ("oklch", ColorSpace::OKLCH),
            ("display-p3", ColorSpace::DisplayP3),
            ("a98rgb", ColorSpace::A98RGB),
            ("prophoto-rgb", ColorSpace::ProPhotoRGB),
            ("rec2020", ColorSpace::Rec2020),
            ("xyz-d65", ColorSpace::XYZD65),
            ("xyz-d50", ColorSpace::XYZD50),
        ];
        require_enum_string_with_mapping(
            ctx,
            path,
            "colorSpace",
            value,
            |v| {
                mapping
                    .iter()
                    .find(|(key, _)| *key == v.to_lowercase())
                    .map(|(_, color_space)| color_space.clone())
            },
            "srgb, srgb-linear, hsl, hwb, cielab, lch, oklab, oklch, display-p3, a98rgb, prophoto-rgb, rec2020, xyz-d65, xyz-d50",
        )
    }
}

define_three_component_color_space!(SRGB, red, green, blue);

define_three_component_color_space!(SRGBLinear, red, green, blue);

define_three_component_color_space!(HSL, hue, saturation, lightness);

define_three_component_color_space!(HWB, hue, whiteness, blackness);

define_three_component_color_space!(CIELAB, lightness, a, b);

define_three_component_color_space!(LCH, lightness, chroma, hue);

define_three_component_color_space!(OKLAB, lightness, a, b);

define_three_component_color_space!(OKLCH, lightness, chroma, hue);

define_three_component_color_space!(DisplayP3, red, green, blue);

define_three_component_color_space!(A98RGB, red, green, blue);

define_three_component_color_space!(ProPhotoRGB, red, green, blue);

define_three_component_color_space!(Rec2020, red, green, blue);

define_three_component_color_space!(XYZD50, x, y, z);

define_three_component_color_space!(XYZD65, x, y, z);

#[derive(Debug, Clone, PartialEq)]
pub struct ColorComponentArray([RefOr<ColorComponentElement<f64>>; 3]);

/// The enum specifies all of the values the "components" key (which is the value of the color tokens) can be
#[derive(Debug, Clone, PartialEq)]
pub enum ColorComponentValue {
    SRGB(SRGB),
    /// sRGB Linear is a linearized version of the sRGB color space. It is used in some design tools to represent colors
    /// in a linear color space.
    /// Components:
    /// - red: The red component of the color, which is a float between 0 and 1
    /// - green: The green component of the color, which is a float between 0 and 1
    /// - blue: The blue component of the color, which is a float between 0 and 1
    SRGBLinear(SRGBLinear),
    /// HSL is a color model that is a polar transformation of the sRGB, supported as early as CSS Color Module 3.
    /// Components:
    /// - hue: The hue component of the color, which is a float from 0 up to (but not including) 360
    /// - saturation: The saturation component of the color, which is a precentage between 0 and 100
    /// - lightness: The lightness component of the color, which is a precentage between 0 and 100
    HSL(HSL),
    /// HWB is a color model that is a polar transformation of the sRGB, supported as early as CSS Color Module 4.
    /// Components:
    /// - hue: The hue component of the color, which is a float from 0 up to (but not including) 360
    /// - whiteness: The whiteness component of the color, which is a precentage between 0 and 100
    /// - blackness: The blackness component of the color, which is a precentage between 0 and 100
    HWB(HWB),
    /// CIELAB is a color space that is designed to be perceptually uniform, meaning that the same amount of numerical change in these values corresponds to about the same amount of visually perceived change
    /// in color. It is used in some design tools to represent colors in a perceptually uniform color space.
    /// Components:
    /// - lightness: The lightness component of the color, which is a float between 0 and 100 representing a percentage
    /// - a: The a component of the color, which is a signed number representing the green-red axis of the color space
    /// - b: The b component of the color, which is a signed number representing the blue-yellow axis of the color space
    /// Note: A and B are theoretically unbounded, but in practice they don't usually exceed the range of -160 to 160
    CIELAB(CIELAB),
    /// LCH is a cylindrical transformation of the CIELAB color space, supported as early as CSS Color Module 4.
    /// Components:
    /// - lightness: The lightness component of the color, which is a float between 0 and 100 representing a percentage
    /// - chroma: The chroma component of the color, which is a non-negative float representing the saturation of the color
    /// - hue: The hue component of the color, which is a float from 0 up to (but not including) 360
    /// Note: chroma is theoretically unbounded, but in practice it doesn't usually exceed the range of 0 to 230
    LCH(LCH),
    /// OKLAB is a color space that is designed to be perceptually uniform, meaning that the same amount of numerical change in these values corresponds to about the same amount of visually perceived change
    /// in color. It is a modernized version of CIELAB that is designed to be more perceptually uniform and to have better support for wide gamut colors. It is used in some design tools to represent colors in a perceptually uniform color space.
    /// Components:
    /// - lightness: The lightness component of the color, which is a float between 0 and 100 representing a percentage
    /// - a: The a component of the color, which is a signed number representing the green-red axis of the color space
    /// - b: The b component of the color, which is a signed number representing the blue-yellow axis of the color space
    /// Note: A and B are theoretically unbounded, but in practice they don't usually exceed the range of -0.5 to 0.5
    OKLAB(OKLAB),
    /// OKLCH is a cylindrical transformation of the OKLAB color space, supported as early as CSS Color Module 4.
    /// Components:
    /// - lightness: The lightness component of the color, which is a float between 0 and 100 representing a percentage
    /// - chroma: The chroma component of the color, which is a non-negative float representing the saturation of the color
    /// - hue: The hue component of the color, which is a float from 0 up to (but not including) 360
    /// Note: chroma is theoretically unbounded, but in practice it doesn't usually exceed the range of 0 to 0.5
    OKLCH(OKLCH),
    /// Display P3 is a wide gamut color space that is designed to encompass the range of colors that can be displayed on modern devices with wide gamut displays. It is used in some design tools
    /// to represent colors in a wide gamut color space.
    /// Components:
    /// - red: The red component of the color, which is a float between 0 and 1
    /// - green: The green component of the color, which is a float between 0 and 1
    /// - blue: The blue component of the color, which is a float between 0 and 1
    DisplayP3(DisplayP3),
    /// A98RGB is a wide gamut color space that is designed to encompass the range of colors that can be displayed on modern devices with wide gamut displays. It is used in some design tools
    /// to represent colors in a wide gamut color space.
    /// Components:
    /// - red: The red component of the color, which is a float between 0 and 1
    /// - green: The green component of the color, which is a float between 0 and 1
    /// - blue: The blue component of the color, which is a float between 0 and 1
    A98RGB(A98RGB),
    /// ProPhoto RGB is a wide gamut color space that is designed to encompass the range of colors that can be displayed on modern devices with wide gamut displays. It is used in some design tools
    /// to represent colors in a wide gamut color space.
    /// Components:
    /// - red: The red component of the color, which is a float between 0 and 1
    /// - green: The green component of the color, which is a float between 0 and 1
    /// - blue: The blue component of the color, which is a float between 0 and 1
    ProPhotoRGB(ProPhotoRGB),
    /// Rec2020 is a wide gamut color space that is designed to encompass the range of colors that can be displayed on modern devices with wide gamut displays. It is used in some design tools
    /// to represent colors in a wide gamut color space.
    /// Components:
    /// - red: The red component of the color, which is a float between 0 and 1
    /// - green: The green component of the color, which is a float between 0 and 1
    /// - blue: The blue component of the color, which is a float between 0 and 1
    Rec2020(Rec2020),
    /// XYZ is a color space that is designed to encompass all colors that can be perceived by the human eye. It is used in some design tools to represent colors in a device-independent color space.
    /// Components:
    /// - x: The x component of the color, which is a float between 0 and 1 representing a percentage of the reference white point's x value
    /// - y: The y component of the color, which is a float between 0 and 1 representing a percentage of the reference white point's y value
    /// - z: The z component of the color, which is a float between 0 and 1 representing a percentage of the reference white point's z value
    XYZD65(XYZD65),
    /// XYZD50 is a variant of the XYZ color space that uses a different reference white point (D50 instead of D65). It is used in some design tools to represent colors in a device-independent color space.
    /// Components:
    /// - x: The x component of the color, which is a float between 0 and 1 representing a percentage of the reference white point's x value
    /// - y: The y component of the color, which is a float between 0 and 1 representing a percentage of the reference white point's y value
    /// - z: The z component of the color, which is a float between 0 and 1 representing a percentage of the reference white point's z value
    XYZD50(XYZD50),
}

impl ColorComponentArray {
    pub fn to_color_component_value(&self, color_space: &ColorSpace) -> RefOr<ColorComponentValue> {
        match color_space {
            ColorSpace::SRGB => {
                RefOr::Literal(ColorComponentValue::SRGB(SRGB::from(self.0.clone())))
            }
            ColorSpace::SRGBLinear => RefOr::Literal(ColorComponentValue::SRGBLinear(
                SRGBLinear::from(self.0.clone()),
            )),
            ColorSpace::HSL => RefOr::Literal(ColorComponentValue::HSL(HSL::from(self.0.clone()))),
            ColorSpace::HWB => RefOr::Literal(ColorComponentValue::HWB(HWB::from(self.0.clone()))),
            ColorSpace::CIELAB => {
                RefOr::Literal(ColorComponentValue::CIELAB(CIELAB::from(self.0.clone())))
            }
            ColorSpace::LCH => RefOr::Literal(ColorComponentValue::LCH(LCH::from(self.0.clone()))),
            ColorSpace::OKLAB => {
                RefOr::Literal(ColorComponentValue::OKLAB(OKLAB::from(self.0.clone())))
            }
            ColorSpace::OKLCH => {
                RefOr::Literal(ColorComponentValue::OKLCH(OKLCH::from(self.0.clone())))
            }
            ColorSpace::DisplayP3 => RefOr::Literal(ColorComponentValue::DisplayP3(
                DisplayP3::from(self.0.clone()),
            )),
            ColorSpace::A98RGB => {
                RefOr::Literal(ColorComponentValue::A98RGB(A98RGB::from(self.0.clone())))
            }
            ColorSpace::ProPhotoRGB => RefOr::Literal(ColorComponentValue::ProPhotoRGB(
                ProPhotoRGB::from(self.0.clone()),
            )),
            ColorSpace::Rec2020 => {
                RefOr::Literal(ColorComponentValue::Rec2020(Rec2020::from(self.0.clone())))
            }
            ColorSpace::XYZD65 => {
                RefOr::Literal(ColorComponentValue::XYZD65(XYZD65::from(self.0.clone())))
            }
            ColorSpace::XYZD50 => {
                RefOr::Literal(ColorComponentValue::XYZD50(XYZD50::from(self.0.clone())))
            }
        }
    }
}

impl<'a> TryFromJson<'a> for ColorComponentArray {
    fn try_from_json(
        ctx: &mut ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        match value {
            serde_json::Value::Array(arr_val) => {
                if arr_val.len() != COLOR_COMPONENT_ARRAY_LENGTH {
                    ctx.push_to_errors(DiagnosticCode::InvalidTokenValue, format!(
                        "Expected an array of length {} for color components, but got an array of length {}",
                        COLOR_COMPONENT_ARRAY_LENGTH,
                        arr_val.len()
                    ), path.into());
                    return None;
                }

                let mut components: [RefOr<ColorComponentElement<f64>>;
                    COLOR_COMPONENT_ARRAY_LENGTH] =
                    core::array::from_fn(|_| RefOr::Literal(ColorComponentElement::None));

                for (index, component) in arr_val.iter().enumerate() {
                    let parsed_val =
                        parse_ref_or_value::<ColorComponentElement<f64>>(ctx, path, component);
                    if let Some(val) = parsed_val {
                        components[index] = val;
                    } else {
                        ctx.push_to_errors(DiagnosticCode::InvalidTokenValue, format!(
                            "Expected a string with the value 'none', a number, or a reference for color components, but got a value of type {}",
                            component
                        ), path.into());
                        return None;
                    }
                }

                Some(ColorComponentArray(components))
            }
            _ => {
                ctx.push_to_errors(DiagnosticCode::InvalidTokenValue, format!(
                    "Expected an array of length {} for color components, but got a value of type {}",
                    COLOR_COMPONENT_ARRAY_LENGTH,
                    value
                ), path.into());
                None
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColorAlphaValue(Option<JsonNumber>);

impl<'a> TryFromJson<'a> for ColorAlphaValue {
    fn try_from_json(
        ctx: &mut ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        match value {
            serde_json::Value::Number(_) => Some(ColorAlphaValue(Some(
                JsonNumber::from_value(value).unwrap(),
            ))),
            serde_json::Value::Null => Some(ColorAlphaValue(None)),
            _ => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidTokenValue,
                    format!(
                        "Expected a number or null for alpha value, but got a value of type {}",
                        value
                    ),
                    path.into(),
                );
                None
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ColorHexValue(HexColorTokenValue);

impl<'a> TryFromJson<'a> for ColorHexValue {
    fn try_from_json(
        ctx: &mut ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        match value {
            serde_json::Value::String(str_val) => Some(ColorHexValue(str_val.clone())),
            _ => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidTokenValue,
                    format!(
                        "Expected a string for hex color token value, but got a value of type {}",
                        value
                    ),
                    path.into(),
                );
                None
            }
        }
    }
}

/// The ColorTokenValue struct represents the value of a color token, which includes the required color space and components of the color token,
/// as well as the optional alpha and hex values of the color token.
#[derive(Debug, Clone, PartialEq)]
pub struct ColorTokenValue {
    /// The color space of the color token, which determines how the components of the color token should be interpreted
    pub color_space: RefOr<ColorSpace>,
    /// The components of the color token, which are interpreted according to the color space of the color token
    pub components: RefOr<ColorComponentArray>,
    /// The alpha component of the color token, which is a float between 0 and 1 representing the opacity of the color.
    /// If the alpha component is not specified, it is assumed to be 1 (fully opaque).
    pub alpha: Option<RefOr<ColorAlphaValue>>,
    /// The hex value of the color token, which is a string representing the color in hexadecimal notation (e.g. "#RRGGBB").
    pub hex: Option<RefOr<ColorHexValue>>,
}

impl<'a> TryFromJson<'a> for ColorTokenValue {
    fn try_from_json(
        ctx: &mut ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        let obj = require_object(ctx, path, value, "color token")?;
        let color_space = obj.required_field::<RefOr<ColorSpace>>(ctx, path, "colorSpace");
        let components = obj.required_field::<RefOr<ColorComponentArray>>(ctx, path, "components");
        let alpha = obj.optional_field::<RefOr<ColorAlphaValue>>(ctx, path, "alpha");
        let hex = obj.optional_field::<RefOr<ColorHexValue>>(ctx, path, "hex");

        match (color_space, components) {
            (Some(color_space), Some(components)) => Some(Self {
                color_space,
                components,
                alpha,
                hex,
            }),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        FileFormat,
        ir::{JsonPointer, JsonRef, RefOr},
    };
    use serde_json::{Number, json};

    fn parser_context() -> ParserContext {
        ParserContext::new("tests.json".into(), FileFormat::Json, String::new())
    }

    #[test]
    fn validates_hex_color_token_values() {
        assert!(validation::validate_hex_color_token_value("#f09"));
        assert!(validation::validate_hex_color_token_value("#f09c"));
        assert!(validation::validate_hex_color_token_value("#ff0099"));
        assert!(!validation::validate_hex_color_token_value("ff0099"));
        assert!(!validation::validate_hex_color_token_value("#ff009"));
        assert!(!validation::validate_hex_color_token_value("#gg0099"));
    }

    #[test]
    fn validates_component_ranges_and_boundaries() {
        let mut ctx = parser_context();

        assert!(validation::validate_range(
            &mut ctx,
            "/token",
            "red",
            &ColorComponentElement::Value(0.5),
            0.0,
            1.0,
            true,
        ));
        assert!(validation::validate_range(
            &mut ctx,
            "/token",
            "red",
            &ColorComponentElement::None,
            0.0,
            1.0,
            true,
        ));
        assert!(!validation::validate_range(
            &mut ctx,
            "/token",
            "hue",
            &ColorComponentElement::Value(360.0),
            0.0,
            360.0,
            false,
        ));

        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(ctx.errors[0].path, "/token");
    }

    #[test]
    fn validates_non_negative_and_alpha_values() {
        let mut ctx = parser_context();

        assert!(validation::validate_non_negative(
            &mut ctx,
            "/token",
            "chroma",
            &ColorComponentElement::Value(0.0),
        ));
        assert!(!validation::validate_non_negative(
            &mut ctx,
            "/token",
            "chroma",
            &ColorComponentElement::Value(-0.01),
        ));
        assert!(validation::validate_alpha(&mut ctx, "/token", Some(1.0)));
        assert!(validation::validate_alpha(&mut ctx, "/token", None));
        assert!(!validation::validate_alpha(&mut ctx, "/token", Some(1.1)));

        assert_eq!(ctx.errors.len(), 2);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(ctx.errors[1].code, DiagnosticCode::InvalidTokenValue);
    }

    #[test]
    fn validates_component_array_helper() {
        let mut ctx = parser_context();
        let components = vec![json!(0.1), json!("none"), json!(1.0)];

        let parsed = validation::validate_component_array(&mut ctx, "/token", &components);

        assert_eq!(
            parsed,
            Some([
                ColorComponentElement::Value(0.1),
                ColorComponentElement::None,
                ColorComponentElement::Value(1.0),
            ])
        );
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn rejects_invalid_component_array_helper_inputs() {
        let mut ctx = parser_context();

        let wrong_length = validation::validate_component_array(&mut ctx, "/token", &[json!(0.1)]);
        let invalid_string = validation::validate_component_array(
            &mut ctx,
            "/token",
            &[json!(0.1), json!("bad"), json!(1.0)],
        );

        assert_eq!(wrong_length, None);
        assert_eq!(invalid_string, None);
        assert_eq!(ctx.errors.len(), 2);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(ctx.errors[1].code, DiagnosticCode::InvalidTokenValue);
    }

    #[test]
    fn parses_color_component_elements() {
        let mut ctx = parser_context();

        let none = ColorComponentElement::<f64>::try_from_json(&mut ctx, "/token", &json!("NoNe"));
        let value = ColorComponentElement::<f64>::try_from_json(&mut ctx, "/token", &json!(0.25));

        assert_eq!(none, Some(ColorComponentElement::None));
        assert_eq!(value, Some(ColorComponentElement::Value(0.25)));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn rejects_invalid_color_component_element() {
        let mut ctx = parser_context();

        let parsed = ColorComponentElement::<f64>::try_from_json(&mut ctx, "/token", &json!(true));

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(ctx.errors[0].path, "/token");
    }

    #[test]
    fn parses_color_space_case_insensitively() {
        let mut ctx = parser_context();

        let parsed = ColorSpace::try_from_json(&mut ctx, "/token/colorSpace", &json!("Display-P3"));

        assert_eq!(parsed, Some(ColorSpace::DisplayP3));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn rejects_unknown_color_space() {
        let mut ctx = parser_context();

        let parsed = ColorSpace::try_from_json(&mut ctx, "/token/colorSpace", &json!("rgb"));

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidEnumValue);
        assert_eq!(ctx.errors[0].path, "/token/colorSpace");
    }

    #[test]
    fn parses_color_component_array_with_literals_and_reference() {
        let mut ctx = parser_context();
        let value = json!([0.1, "none", { "$ref": "#/palette/blue" }]);

        let parsed = ColorComponentArray::try_from_json(&mut ctx, "/token/components", &value);

        assert_eq!(
            parsed,
            Some(ColorComponentArray([
                RefOr::Literal(ColorComponentElement::Value(0.1)),
                RefOr::Literal(ColorComponentElement::None),
                RefOr::Ref(JsonRef::new_local_pointer(
                    "#/palette/blue".into(),
                    JsonPointer::from("#/palette/blue"),
                )),
            ]))
        );
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn rejects_color_component_array_with_invalid_length() {
        let mut ctx = parser_context();

        let parsed =
            ColorComponentArray::try_from_json(&mut ctx, "/token/components", &json!([0.1, 0.2]));

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(ctx.errors[0].path, "/token/components");
    }

    #[test]
    fn rejects_color_component_array_with_invalid_item_type() {
        let mut ctx = parser_context();

        let parsed = ColorComponentArray::try_from_json(
            &mut ctx,
            "/token/components",
            &json!([0.1, true, 0.3]),
        );

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 2);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(ctx.errors[0].path, "/token/components");
        assert_eq!(ctx.errors[1].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(ctx.errors[1].path, "/token/components");
    }

    #[test]
    fn rejects_color_component_array_with_invalid_reference() {
        let mut ctx = parser_context();

        let parsed = ColorComponentArray::try_from_json(
            &mut ctx,
            "/token/components",
            &json!([0.1, { "$ref": "not-a-pointer" }, 0.3]),
        );

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 2);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidReference);
        assert_eq!(ctx.errors[0].path, "/token/components");
        assert_eq!(ctx.errors[1].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(ctx.errors[1].path, "/token/components");
    }

    #[test]
    fn parses_alpha_and_hex_values() {
        let mut ctx = parser_context();

        let alpha = ColorAlphaValue::try_from_json(&mut ctx, "/token/alpha", &json!(0.5));
        let alpha_null = ColorAlphaValue::try_from_json(&mut ctx, "/token/alpha", &json!(null));
        let hex = ColorHexValue::try_from_json(&mut ctx, "/token/hex", &json!("#ff0099"));

        assert_eq!(
            alpha,
            Some(ColorAlphaValue(Some(JsonNumber(
                Number::from_f64(0.5).unwrap()
            ))))
        );
        assert_eq!(alpha_null, Some(ColorAlphaValue(None)));
        assert_eq!(hex, Some(ColorHexValue("#ff0099".into())));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn rejects_invalid_alpha_and_hex_types() {
        let mut ctx = parser_context();

        let alpha = ColorAlphaValue::try_from_json(&mut ctx, "/token/alpha", &json!("opaque"));
        let hex = ColorHexValue::try_from_json(&mut ctx, "/token/hex", &json!(42));

        assert_eq!(alpha, None);
        assert_eq!(hex, None);
        assert_eq!(ctx.errors.len(), 2);
        assert_eq!(ctx.errors[0].path, "/token/alpha");
        assert_eq!(ctx.errors[1].path, "/token/hex");
    }

    #[test]
    fn converts_component_array_to_color_component_value() {
        let array = ColorComponentArray([
            RefOr::Literal(ColorComponentElement::Value(0.1)),
            RefOr::Literal(ColorComponentElement::Value(0.2)),
            RefOr::Literal(ColorComponentElement::Value(0.3)),
        ]);

        let srgb = array.to_color_component_value(&ColorSpace::SRGB);
        let xyz = array.to_color_component_value(&ColorSpace::XYZD65);

        assert!(matches!(srgb, RefOr::Literal(ColorComponentValue::SRGB(_))));
        assert!(matches!(
            xyz,
            RefOr::Literal(ColorComponentValue::XYZD65(_))
        ));
    }

    #[test]
    fn parses_color_token_with_all_fields() {
        let mut ctx = parser_context();
        let value = json!({
            "colorSpace": "srgb",
            "components": [0.1, 0.2, 0.3],
            "alpha": 0.5,
            "hex": "#19334d"
        });

        let parsed = ColorTokenValue::try_from_json(&mut ctx, "/token", &value);

        assert_eq!(
            parsed,
            Some(ColorTokenValue {
                color_space: RefOr::Literal(ColorSpace::SRGB),
                components: RefOr::Literal(ColorComponentArray([
                    RefOr::Literal(ColorComponentElement::Value(0.1)),
                    RefOr::Literal(ColorComponentElement::Value(0.2)),
                    RefOr::Literal(ColorComponentElement::Value(0.3)),
                ])),
                alpha: Some(RefOr::Literal(ColorAlphaValue(Some(JsonNumber(
                    Number::from_f64(0.5).unwrap(),
                ))))),
                hex: Some(RefOr::Literal(ColorHexValue("#19334d".into()))),
            })
        );
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn parses_color_token_with_references_for_fields() {
        let mut ctx = parser_context();
        let value = json!({
            "colorSpace": { "$ref": "#/palette/space" },
            "components": { "$ref": "#/palette/components" },
            "alpha": { "$ref": "#/palette/alpha" },
            "hex": { "$ref": "#/palette/hex" }
        });

        let parsed = ColorTokenValue::try_from_json(&mut ctx, "/token", &value).unwrap();

        match parsed.color_space {
            RefOr::Ref(json_ref) => assert_eq!(
                json_ref,
                JsonRef::new_local_pointer(
                    "#/palette/space".to_string(),
                    JsonPointer::from("#/palette/space")
                )
            ),
            _ => panic!("expected colorSpace to be a reference"),
        }
        match parsed.components {
            RefOr::Ref(json_ref) => assert_eq!(
                json_ref,
                JsonRef::new_local_pointer(
                    "#/palette/components".to_string(),
                    JsonPointer::from("#/palette/components")
                )
            ),
            _ => panic!("expected components to be a reference"),
        }
        match parsed.alpha {
            Some(RefOr::Ref(json_ref)) => assert_eq!(
                json_ref,
                JsonRef::new_local_pointer(
                    "#/palette/alpha".to_string(),
                    JsonPointer::from("#/palette/alpha")
                )
            ),
            _ => panic!("expected alpha to be a reference"),
        }
        match parsed.hex {
            Some(RefOr::Ref(json_ref)) => assert_eq!(
                json_ref,
                JsonRef::new_local_pointer(
                    "#/palette/hex".to_string(),
                    JsonPointer::from("#/palette/hex")
                )
            ),
            _ => panic!("expected hex to be a reference"),
        }
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn reports_missing_required_color_token_fields() {
        let mut ctx = parser_context();
        let value = json!({});

        let parsed = ColorTokenValue::try_from_json(&mut ctx, "/token", &value);

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 2);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[0].path, "/token/colorSpace");
        assert_eq!(ctx.errors[1].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[1].path, "/token/components");
    }

    #[test]
    fn preserves_color_token_when_optional_fields_are_invalid() {
        let mut ctx = parser_context();
        let value = json!({
            "colorSpace": "srgb",
            "components": [0.1, 0.2, 0.3],
            "alpha": "opaque",
            "hex": 42
        });

        let parsed = ColorTokenValue::try_from_json(&mut ctx, "/token", &value);

        assert_eq!(
            parsed,
            Some(ColorTokenValue {
                color_space: RefOr::Literal(ColorSpace::SRGB),
                components: RefOr::Literal(ColorComponentArray([
                    RefOr::Literal(ColorComponentElement::Value(0.1)),
                    RefOr::Literal(ColorComponentElement::Value(0.2)),
                    RefOr::Literal(ColorComponentElement::Value(0.3)),
                ])),
                alpha: None,
                hex: None,
            })
        );
        assert_eq!(ctx.errors.len(), 2);
        assert_eq!(ctx.errors[0].path, "/token/alpha");
        assert_eq!(ctx.errors[1].path, "/token/hex");
    }

    #[test]
    fn rejects_invalid_top_level_color_token_shape() {
        let mut ctx = parser_context();

        let parsed = ColorTokenValue::try_from_json(&mut ctx, "/token", &json!("#fff"));

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyType);
        assert_eq!(ctx.errors[0].path, "/token");
    }
}
