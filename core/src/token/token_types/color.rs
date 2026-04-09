//! The `color` module defines the data structures for color tokens defined in the DTCG specification

use std::f64;

use crate::{
    ParserContext,
    errors::DiagnosticCode,
    ir::{RefOr, parse_ref_or_value},
    token::{
        ParseState, TryFromJson, TryFromJsonField,
        utils::{
            FieldPresence, FloatOrInteger, parse_field, parse_field_no_ref,
            require_enum_string_with_mapping,
        },
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

impl<'a> TryFromJsonField<'a> for ColorComponentElement<f64> {
    fn try_from_json_field(
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

impl<'a> TryFromJsonField<'a> for ColorSpace {
    fn try_from_json_field(
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
    pub fn to_color_component_value(&self, color_space: &ColorSpace) -> ColorComponentValue {
        match color_space {
            ColorSpace::SRGB => ColorComponentValue::SRGB(SRGB::from(self.0.clone())),
            ColorSpace::SRGBLinear => {
                ColorComponentValue::SRGBLinear(SRGBLinear::from(self.0.clone()))
            }
            ColorSpace::HSL => ColorComponentValue::HSL(HSL::from(self.0.clone())),
            ColorSpace::HWB => ColorComponentValue::HWB(HWB::from(self.0.clone())),
            ColorSpace::CIELAB => ColorComponentValue::CIELAB(CIELAB::from(self.0.clone())),
            ColorSpace::LCH => ColorComponentValue::LCH(LCH::from(self.0.clone())),
            ColorSpace::OKLAB => ColorComponentValue::OKLAB(OKLAB::from(self.0.clone())),
            ColorSpace::OKLCH => ColorComponentValue::OKLCH(OKLCH::from(self.0.clone())),
            ColorSpace::DisplayP3 => {
                ColorComponentValue::DisplayP3(DisplayP3::from(self.0.clone()))
            }
            ColorSpace::A98RGB => ColorComponentValue::A98RGB(A98RGB::from(self.0.clone())),
            ColorSpace::ProPhotoRGB => {
                ColorComponentValue::ProPhotoRGB(ProPhotoRGB::from(self.0.clone()))
            }
            ColorSpace::Rec2020 => ColorComponentValue::Rec2020(Rec2020::from(self.0.clone())),
            ColorSpace::XYZD65 => ColorComponentValue::XYZD65(XYZD65::from(self.0.clone())),
            ColorSpace::XYZD50 => ColorComponentValue::XYZD50(XYZD50::from(self.0.clone())),
        }
    }
}

impl<'a> TryFromJsonField<'a> for ColorComponentArray {
    fn try_from_json_field(
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
pub struct ColorAlphaValue(Option<FloatOrInteger>);

impl<'a> TryFromJsonField<'a> for ColorAlphaValue {
    fn try_from_json_field(
        ctx: &mut ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        match value {
            serde_json::Value::Number(num_val) => {
                if let Some(float_val) = num_val.as_f64() {
                    Some(ColorAlphaValue(Some(FloatOrInteger::Float(float_val))))
                } else if let Some(int_val) = num_val.as_i64() {
                    Some(ColorAlphaValue(Some(FloatOrInteger::Integer(int_val))))
                } else {
                    ctx.push_to_errors(DiagnosticCode::InvalidTokenValue, format!(
                        "Expected a number for alpha value, but got a number that cannot be represented as f64 or i64: {}",
                        num_val
                    ), path.into());
                    None
                }
            }
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

impl<'a> TryFromJsonField<'a> for ColorHexValue {
    fn try_from_json_field(
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
    pub color_space: ColorSpace,
    /// The components of the color token, which are interpreted according to the color space of the color token
    pub components: RefOr<ColorComponentArray>,
    /// The alpha component of the color token, which is a float between 0 and 1 representing the opacity of the color.
    /// If the alpha component is not specified, it is assumed to be 1 (fully opaque).
    pub alpha: Option<RefOr<ColorAlphaValue>>,
    /// The hex value of the color token, which is a string representing the color in hexadecimal notation (e.g. "#RRGGBB").
    pub hex: Option<ColorHexValue>,
}

impl<'a> TryFromJson<'a> for ColorTokenValue {
    fn try_from_json(
        ctx: &mut ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        // The DTCG specification defines that a color token value shall be an object with the following properties:
        // - "color_space": A required string that specifies the color space of the color token, which determines how the components of the color token should be interpreted
        // - "components": A required object that specifies the components of the color token, which are interpreted according to the color space of the color token
        // - "alpha": An optional number between 0 and 1 representing the opacity of the color. If the alpha component is not specified, it is assumed to be 1 (fully opaque)
        // - "hex": An optional string representing the color in hexadecimal notation (e.g. "#RRGGBB"). If the hex value is specified, it must be a valid hex color string and the color components of the color token must match the hex value of the color token
        match value {
            serde_json::Value::Object(map) => {
                let color_space = parse_field_no_ref::<ColorSpace>(
                    ctx,
                    path,
                    map,
                    "colorSpace",
                    FieldPresence::Required,
                );
                let color_components = parse_field::<ColorComponentArray>(
                    ctx,
                    path,
                    map,
                    "components",
                    FieldPresence::Required,
                );
                let alpha = parse_field::<ColorAlphaValue>(
                    ctx,
                    path,
                    map,
                    "alpha",
                    FieldPresence::Optional,
                );
                let hex = parse_field_no_ref::<ColorHexValue>(
                    ctx,
                    path,
                    map,
                    "hex",
                    FieldPresence::Optional,
                );

                match (color_space, color_components) {
                    (Some(color_space), Some(color_components)) => ParseState::Parsed(Self {
                        color_space,
                        components: color_components,
                        alpha,
                        hex,
                    }),
                    _ => {
                        // If either the color space or the color components are missing or invalid, we cannot parse the color token value, so we return a fatal error
                        ParseState::Skipped
                    }
                }
            }
            _ => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidTokenValue,
                    format!(
                        "Expected an object for color token value, but found {}",
                        value
                    ),
                    path.to_string(),
                );
                // We return a fatal error here as we should not have made it here, as an earlier check should have
                // verified if this is an object or not and returned an error if it wasn't, so if we are in this branch it means there is a bug in our code
                ParseState::Skipped
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        FileFormat,
        errors::DiagnosticCode,
        ir::{JsonPointer, JsonRef, RefOr},
        token::utils::FloatOrInteger,
    };
    use serde_json::json;

    fn make_context() -> ParserContext {
        ParserContext::new(String::from("test.json"), FileFormat::Json, String::new())
    }

    fn parse_color(value: &serde_json::Value) -> (ParseState<ColorTokenValue>, ParserContext) {
        let mut ctx = make_context();
        let result = ColorTokenValue::try_from_json(&mut ctx, "tokens.colors.primary", value);
        (result, ctx)
    }

    fn expect_parsed(result: ParseState<ColorTokenValue>) -> ColorTokenValue {
        let ParseState::Parsed(parsed) = result else {
            panic!("expected color token to parse successfully");
        };

        parsed
    }

    fn matches_color_space_and_components(value: &ColorTokenValue) -> bool {
        let RefOr::Literal(components) = &value.components else {
            return false;
        };

        match (
            &value.color_space,
            components.to_color_component_value(&value.color_space),
        ) {
            (ColorSpace::SRGB, ColorComponentValue::SRGB(_)) => true,
            (ColorSpace::SRGBLinear, ColorComponentValue::SRGBLinear(_)) => true,
            (ColorSpace::HSL, ColorComponentValue::HSL(_)) => true,
            (ColorSpace::HWB, ColorComponentValue::HWB(_)) => true,
            (ColorSpace::CIELAB, ColorComponentValue::CIELAB(_)) => true,
            (ColorSpace::LCH, ColorComponentValue::LCH(_)) => true,
            (ColorSpace::OKLAB, ColorComponentValue::OKLAB(_)) => true,
            (ColorSpace::OKLCH, ColorComponentValue::OKLCH(_)) => true,
            (ColorSpace::DisplayP3, ColorComponentValue::DisplayP3(_)) => true,
            (ColorSpace::A98RGB, ColorComponentValue::A98RGB(_)) => true,
            (ColorSpace::ProPhotoRGB, ColorComponentValue::ProPhotoRGB(_)) => true,
            (ColorSpace::Rec2020, ColorComponentValue::Rec2020(_)) => true,
            (ColorSpace::XYZD65, ColorComponentValue::XYZD65(_)) => true,
            (ColorSpace::XYZD50, ColorComponentValue::XYZD50(_)) => true,
            _ => false,
        }
    }

    #[test]
    fn parses_valid_srgb_color_with_optional_fields() {
        let value = json!({
            "colorSpace": "srgb",
            "components": [0.25, 0.5, 0.75],
            "alpha": 0.4,
            "hex": "#abc"
        });

        let (result, ctx) = parse_color(&value);

        assert!(ctx.errors.is_empty());

        let parsed = expect_parsed(result);

        assert!(matches!(parsed.color_space, ColorSpace::SRGB));
        assert_eq!(
            parsed.components,
            RefOr::Literal(ColorComponentArray([
                RefOr::Literal(ColorComponentElement::Value(0.25)),
                RefOr::Literal(ColorComponentElement::Value(0.5)),
                RefOr::Literal(ColorComponentElement::Value(0.75)),
            ]))
        );
        assert_eq!(
            parsed.alpha,
            Some(RefOr::Literal(ColorAlphaValue(Some(
                FloatOrInteger::Float(0.4,)
            ))))
        );
        assert_eq!(parsed.hex, Some(ColorHexValue("#abc".to_string())));
    }

    #[test]
    fn parses_none_component_for_lch_color() {
        let value = json!({
            "colorSpace": "lch",
            "components": [50.0, "none", 180.0]
        });

        let (result, ctx) = parse_color(&value);

        assert!(ctx.errors.is_empty());

        let parsed = expect_parsed(result);

        assert_eq!(parsed.color_space, ColorSpace::LCH);
        assert_eq!(
            parsed.components,
            RefOr::Literal(ColorComponentArray([
                RefOr::Literal(ColorComponentElement::Value(50.0)),
                RefOr::Literal(ColorComponentElement::None),
                RefOr::Literal(ColorComponentElement::Value(180.0)),
            ]))
        );
    }

    #[test]
    fn parses_components_and_alpha_as_references() {
        let value = json!({
            "colorSpace": "srgb",
            "components": { "$ref": "#/tokens/colors/base/components" },
            "alpha": { "$ref": "" }
        });

        let (result, ctx) = parse_color(&value);

        assert!(ctx.errors.is_empty());

        let parsed = expect_parsed(result);

        assert_eq!(parsed.color_space, ColorSpace::SRGB);
        assert_eq!(
            parsed.components,
            RefOr::Ref(JsonRef::new_local_pointer(
                "#/tokens/colors/base/components".to_string(),
                JsonPointer::from("#/tokens/colors/base/components"),
            ))
        );
        assert_eq!(
            parsed.alpha,
            Some(RefOr::Ref(JsonRef::new_local_pointer(
                String::new(),
                JsonPointer::new(),
            )))
        );
        assert_eq!(parsed.hex, None);
    }

    #[test]
    fn parses_all_supported_color_spaces_with_valid_components() {
        let cases = [
            ("srgb", ColorSpace::SRGB, json!([0.0, 1.0, 0.5])),
            (
                "srgb-linear",
                ColorSpace::SRGBLinear,
                json!([0.0, 1.0, 0.5]),
            ),
            ("hsl", ColorSpace::HSL, json!([359.0, 0.0, 100.0])),
            ("hwb", ColorSpace::HWB, json!([0.0, 0.0, 100.0])),
            ("cielab", ColorSpace::CIELAB, json!([0.0, -50.0, 50.0])),
            ("lch", ColorSpace::LCH, json!([100.0, 0.0, 359.0])),
            ("oklab", ColorSpace::OKLAB, json!([50.0, -0.5, 0.5])),
            ("oklch", ColorSpace::OKLCH, json!([100.0, 0.0, 359.0])),
            ("display-p3", ColorSpace::DisplayP3, json!([0.0, 1.0, 0.5])),
            ("a98rgb", ColorSpace::A98RGB, json!([0.0, 1.0, 0.5])),
            (
                "prophoto-rgb",
                ColorSpace::ProPhotoRGB,
                json!([0.0, 1.0, 0.5]),
            ),
            ("rec2020", ColorSpace::Rec2020, json!([0.0, 1.0, 0.5])),
            ("xyz-d65", ColorSpace::XYZD65, json!([0.0, 1.0, 0.5])),
            ("xyz-d50", ColorSpace::XYZD50, json!([0.0, 1.0, 0.5])),
        ];

        for (color_space_name, expected_color_space, components) in cases {
            let value = json!({
                "colorSpace": color_space_name,
                "components": components
            });

            let (result, ctx) = parse_color(&value);

            assert!(
                ctx.errors.is_empty(),
                "unexpected diagnostics for {color_space_name}"
            );

            let parsed = expect_parsed(result);

            assert_eq!(parsed.color_space, expected_color_space);
            assert!(
                matches_color_space_and_components(&parsed),
                "expected color space and components to match for {color_space_name}"
            );
        }
    }

    #[test]
    fn skips_when_required_fields_are_missing() {
        let value = json!({});

        let (result, ctx) = parse_color(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 2);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[0].message, "Missing required field: colorSpace");
        assert_eq!(ctx.errors[1].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[1].message, "Missing required field: components");
    }

    #[test]
    fn skips_when_color_space_is_invalid() {
        let value = json!({
            "colorSpace": "ansi-rgb",
            "components": [0.1, 0.2, 0.3]
        });

        let (result, ctx) = parse_color(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidEnumValue);
        assert_eq!(
            ctx.errors[0].message,
            "Expected one of srgb, srgb-linear, hsl, hwb, cielab, lch, oklab, oklch, display-p3, a98rgb, prophoto-rgb, rec2020, xyz-d65, xyz-d50 for the field 'colorSpace', but got 'ansi-rgb'"
        );
        assert_eq!(ctx.errors[0].path, "tokens.colors.primary");
    }

    #[test]
    fn skips_when_components_field_has_wrong_type() {
        let value = json!({
            "colorSpace": "srgb",
            "components": true
        });

        let (result, ctx) = parse_color(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(
            ctx.errors[0].message,
            "Expected an array of length 3 for color components, but got a value of type true"
        );
        assert_eq!(ctx.errors[0].path, "tokens.colors.primary");
    }

    #[test]
    fn skips_when_component_array_has_wrong_length() {
        let value = json!({
            "colorSpace": "srgb",
            "components": [0.1, 0.2]
        });

        let (result, ctx) = parse_color(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(
            ctx.errors[0].message,
            "Expected an array of length 3 for color components, but got an array of length 2"
        );
        assert_eq!(ctx.errors[0].path, "tokens.colors.primary");
    }

    #[test]
    fn skips_when_component_keyword_is_not_none_and_reports_follow_up_error() {
        let value = json!({
            "colorSpace": "srgb",
            "components": [0.1, "invalid", 0.3]
        });

        let (result, ctx) = parse_color(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 2);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(
            ctx.errors[0].message,
            "Expected a string with the value 'none' or a number for color components, but got a string with the value 'invalid'"
        );
        assert_eq!(ctx.errors[0].path, "tokens.colors.primary");
        assert_eq!(ctx.errors[1].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(
            ctx.errors[1].message,
            "Expected a string with the value 'none', a number, or a reference for color components, but got a value of type \"invalid\""
        );
        assert_eq!(ctx.errors[1].path, "tokens.colors.primary");
    }

    #[test]
    fn skips_when_component_reference_is_invalid_and_reports_follow_up_error() {
        let value = json!({
            "colorSpace": "srgb",
            "components": [0.1, { "$ref": "tokens/colors/base/components/1" }, 0.3]
        });

        let (result, ctx) = parse_color(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 2);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidReference);
        assert_eq!(
            ctx.errors[0].message,
            "Invalid JSON pointer: tokens/colors/base/components/1"
        );
        assert_eq!(ctx.errors[0].path, "tokens.colors.primary");
        assert_eq!(ctx.errors[1].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(
            ctx.errors[1].message,
            "Expected a string with the value 'none', a number, or a reference for color components, but got a value of type {\"$ref\":\"tokens/colors/base/components/1\"}"
        );
        assert_eq!(ctx.errors[1].path, "tokens.colors.primary");
    }

    #[test]
    fn keeps_parsing_when_alpha_is_out_of_range_because_no_validation_runs() {
        let value = json!({
            "colorSpace": "srgb",
            "components": [0.1, 0.2, 0.3],
            "alpha": 1.5
        });

        let (result, ctx) = parse_color(&value);
        let parsed = expect_parsed(result);

        assert!(ctx.errors.is_empty());
        assert_eq!(
            parsed.alpha,
            Some(RefOr::Literal(ColorAlphaValue(Some(
                FloatOrInteger::Float(1.5,)
            ))))
        );
    }

    #[test]
    fn keeps_parsing_when_alpha_has_wrong_json_type() {
        let value = json!({
            "colorSpace": "srgb",
            "components": [0.1, 0.2, 0.3],
            "alpha": "opaque"
        });

        let (result, ctx) = parse_color(&value);

        let parsed = expect_parsed(result);

        assert_eq!(parsed.alpha, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(
            ctx.errors[0].message,
            "Expected a number or null for alpha value, but got a value of type \"opaque\""
        );
        assert_eq!(ctx.errors[0].path, "tokens.colors.primary");
    }

    #[test]
    fn keeps_parsing_when_hex_contents_are_not_validated() {
        let value = json!({
            "colorSpace": "srgb",
            "components": [0.1, 0.2, 0.3],
            "hex": "#12"
        });

        let (result, ctx) = parse_color(&value);

        let parsed = expect_parsed(result);

        assert!(ctx.errors.is_empty());
        assert_eq!(parsed.hex, Some(ColorHexValue("#12".to_string())));
    }

    #[test]
    fn keeps_parsing_when_hex_has_wrong_json_type() {
        let value = json!({
            "colorSpace": "srgb",
            "components": [0.1, 0.2, 0.3],
            "hex": false
        });

        let (result, ctx) = parse_color(&value);

        let parsed = expect_parsed(result);

        assert_eq!(parsed.hex, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(
            ctx.errors[0].message,
            "Expected a string for hex color token value, but got a value of type false"
        );
        assert_eq!(ctx.errors[0].path, "tokens.colors.primary");
    }

    #[test]
    fn returns_error_for_non_object_value_and_records_diagnostic() {
        let value = json!("#ff0099");

        let (result, ctx) = parse_color(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(
            ctx.errors[0].message,
            "Expected an object for color token value, but found \"#ff0099\""
        );
        assert_eq!(ctx.errors[0].path, "tokens.colors.primary");
    }

    #[test]
    fn validates_hex_color_formats() {
        assert!(validation::validate_hex_color_token_value("#f09"));
        assert!(validation::validate_hex_color_token_value("#f09c"));
        assert!(validation::validate_hex_color_token_value("#ff0099"));
        assert!(!validation::validate_hex_color_token_value("ff0099"));
        assert!(!validation::validate_hex_color_token_value("#ff"));
        assert!(!validation::validate_hex_color_token_value("#ggg"));
    }

    #[test]
    fn validates_component_arrays_directly() {
        let mut ctx = make_context();

        let parsed = validation::validate_component_array(
            &mut ctx,
            "tokens.colors.primary",
            &[json!(0.1), json!("none"), json!(0.3)],
        )
        .expect("expected valid component array");

        assert!(matches!(parsed[0], ColorComponentElement::Value(0.1)));
        assert!(matches!(parsed[1], ColorComponentElement::None));
        assert!(matches!(parsed[2], ColorComponentElement::Value(0.3)));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn rejects_component_arrays_with_invalid_value_types() {
        let mut ctx = make_context();

        let parsed = validation::validate_component_array(
            &mut ctx,
            "tokens.colors.primary",
            &[json!(0.1), json!(true), json!(0.3)],
        );

        assert!(parsed.is_none());
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(
            ctx.errors[0].message,
            "Expected a string with the value 'none' or a number for color components, but got a value of type true"
        );
    }

    #[test]
    fn validate_range_handles_none_inclusive_and_exclusive_bounds() {
        let mut ctx = make_context();

        assert!(validation::validate_range(
            &mut ctx,
            "tokens.colors.primary",
            "red",
            &ColorComponentElement::None,
            0.0,
            1.0,
            true,
        ));
        assert!(validation::validate_range(
            &mut ctx,
            "tokens.colors.primary",
            "red",
            &ColorComponentElement::Value(1.0),
            0.0,
            1.0,
            true,
        ));
        assert!(!validation::validate_range(
            &mut ctx,
            "tokens.colors.primary",
            "hue",
            &ColorComponentElement::Value(360.0),
            0.0,
            360.0,
            false,
        ));

        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(
            ctx.errors[0].message,
            "Invalid color component value at path 'tokens.colors.primary.hue': Value must be between 0 and 360 (exclusive)"
        );
    }

    #[test]
    fn validate_non_negative_and_alpha_cover_boundaries() {
        let mut ctx = make_context();

        assert!(validation::validate_non_negative(
            &mut ctx,
            "tokens.colors.primary",
            "chroma",
            &ColorComponentElement::Value(0.0),
        ));
        assert!(!validation::validate_non_negative(
            &mut ctx,
            "tokens.colors.primary",
            "chroma",
            &ColorComponentElement::Value(-0.1),
        ));
        assert!(validation::validate_alpha(
            &mut ctx,
            "tokens.colors.primary",
            None
        ));
        assert!(validation::validate_alpha(
            &mut ctx,
            "tokens.colors.primary",
            Some(0.0),
        ));
        assert!(validation::validate_alpha(
            &mut ctx,
            "tokens.colors.primary",
            Some(1.0),
        ));
        assert!(!validation::validate_alpha(
            &mut ctx,
            "tokens.colors.primary",
            Some(-0.01),
        ));

        assert_eq!(ctx.errors.len(), 2);
        assert_eq!(
            ctx.errors[0].message,
            "Invalid color component value at path 'tokens.colors.primary.chroma': Value must be non-negative"
        );
        assert_eq!(
            ctx.errors[1].message,
            "Invalid color component value at path 'tokens.colors.primary.alpha': Alpha value must be between 0 and 1 (inclusive)"
        );
    }
}
