//! The `color` module defines the data structures for color tokens defined in the DTCG specification

use core::f64;

use crate::{
    ParserContext,
    errors::DiagnosticCode,
    token::{
        ParseState, TryFromJson,
        ir::RefOr,
        utils::{
            require_array, require_enum_string_with_mapping, require_number, require_object_field,
            require_string,
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
        #[derive(Debug, Clone)]
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
#[derive(Debug, Clone)]
pub enum ColorComponentElement<T> {
    Value(T),
    None,
}

/// The DTCG specification defines the valid color spaces that can be used in color tokens.
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

pub trait ValidateColorComponentValue {
    /// Validates that the color component values of a color token are valid according to the DTCG specification.
    /// This method is typically implemented by the color component structs (e.g. `SRGB`, `HSL`, etc.) and is called by the `validate` method of the `ColorTokenValue` struct to validate the color component values of a color token.
    ///
    /// # Arguments
    ///
    /// - `ctx` - The parser context to push errors to if the color component values are invalid
    /// - `path` - The path to the color token in the token file (e.g. "tokens.colors.primary"), which is used in error messages when pushing errors to the parser context
    ///
    /// # Returns
    ///
    /// - `true` if the color component values are valid, `false` otherwise
    fn validate(&self, ctx: &mut ParserContext, path: &str) -> bool;
}

define_three_component_color_space!(SRGB, red, green, blue);

/* impl ValidateColorComponentValue for SRGB {
    fn validate(&self, ctx: &mut ParserContext, path: &str) -> bool {
        let mut is_valid = true;
        is_valid &= validation::validate_range(ctx, path, "red", &self.red, 0.0, 1.0, true);
        is_valid &= validation::validate_range(ctx, path, "green", &self.green, 0.0, 1.0, true);
        is_valid &= validation::validate_range(ctx, path, "blue", &self.blue, 0.0, 1.0, true);
        is_valid
    }
} */

define_three_component_color_space!(SRGBLinear, red, green, blue);

/* impl ValidateColorComponentValue for SRGBLinear {
    fn validate(&self, ctx: &mut ParserContext, path: &str) -> bool {
        let mut is_valid = true;
        is_valid &= validation::validate_range(ctx, path, "red", &self.red, 0.0, 1.0, true);
        is_valid &= validation::validate_range(ctx, path, "green", &self.green, 0.0, 1.0, true);
        is_valid &= validation::validate_range(ctx, path, "blue", &self.blue, 0.0, 1.0, true);
        is_valid
    }
} */

define_three_component_color_space!(HSL, hue, saturation, lightness);

/* impl ValidateColorComponentValue for HSL {
    fn validate(&self, ctx: &mut ParserContext, path: &str) -> bool {
        let mut is_valid = true;
        is_valid &= validation::validate_range(ctx, path, "hue", &self.hue, 0.0, 360.0, false);
        is_valid &=
            validation::validate_range(ctx, path, "saturation", &self.saturation, 0.0, 100.0, true);
        is_valid &=
            validation::validate_range(ctx, path, "lightness", &self.lightness, 0.0, 100.0, true);
        is_valid
    }
} */

define_three_component_color_space!(HWB, hue, whiteness, blackness);

/* impl ValidateColorComponentValue for HWB {
    fn validate(&self, ctx: &mut ParserContext, path: &str) -> bool {
        let mut is_valid = true;
        is_valid &= validation::validate_range(ctx, path, "hue", &self.hue, 0.0, 360.0, false);
        is_valid &=
            validation::validate_range(ctx, path, "whiteness", &self.whiteness, 0.0, 100.0, true);
        is_valid &=
            validation::validate_range(ctx, path, "blackness", &self.blackness, 0.0, 100.0, true);
        is_valid
    }
} */

define_three_component_color_space!(CIELAB, lightness, a, b);

/* impl ValidateColorComponentValue for CIELAB {
    fn validate(&self, ctx: &mut ParserContext, path: &str) -> bool {
        let mut is_valid = true;
        is_valid &=
            validation::validate_range(ctx, path, "lightness", &self.lightness, 0.0, 100.0, true);
        is_valid &= validation::validate_range(
            ctx,
            path,
            "a",
            &self.a,
            f64::NEG_INFINITY,
            f64::INFINITY,
            false,
        );
        is_valid &= validation::validate_range(
            ctx,
            path,
            "b",
            &self.b,
            f64::NEG_INFINITY,
            f64::INFINITY,
            false,
        );
        is_valid
    }
} */

define_three_component_color_space!(LCH, lightness, chroma, hue);

/* impl ValidateColorComponentValue for LCH {
    fn validate(&self, ctx: &mut ParserContext, path: &str) -> bool {
        let mut is_valid = true;
        is_valid &=
            validation::validate_range(ctx, path, "lightness", &self.lightness, 0.0, 100.0, true);
        is_valid &= validation::validate_non_negative(ctx, path, "chroma", &self.chroma);
        is_valid &= validation::validate_range(ctx, path, "hue", &self.hue, 0.0, 360.0, false);
        is_valid
    }
} */

define_three_component_color_space!(OKLAB, lightness, a, b);

/* impl ValidateColorComponentValue for OKLAB {
    fn validate(&self, ctx: &mut ParserContext, path: &str) -> bool {
        let mut is_valid = true;
        is_valid &=
            validation::validate_range(ctx, path, "lightness", &self.lightness, 0.0, 100.0, true);
        is_valid &= validation::validate_range(
            ctx,
            path,
            "a",
            &self.a,
            f64::NEG_INFINITY,
            f64::INFINITY,
            false,
        );
        is_valid &= validation::validate_range(
            ctx,
            path,
            "b",
            &self.b,
            f64::NEG_INFINITY,
            f64::INFINITY,
            false,
        );
        is_valid
    }
} */

define_three_component_color_space!(OKLCH, lightness, chroma, hue);

/* impl ValidateColorComponentValue for OKLCH {
    fn validate(&self, ctx: &mut ParserContext, path: &str) -> bool {
        let mut is_valid = true;
        is_valid &=
            validation::validate_range(ctx, path, "lightness", &self.lightness, 0.0, 100.0, true);
        is_valid &= validation::validate_non_negative(ctx, path, "chroma", &self.chroma);
        is_valid &= validation::validate_range(ctx, path, "hue", &self.hue, 0.0, 360.0, false);
        is_valid
    }
} */

define_three_component_color_space!(DisplayP3, red, green, blue);

/* impl ValidateColorComponentValue for DisplayP3 {
    fn validate(&self, ctx: &mut ParserContext, path: &str) -> bool {
        let mut is_valid = true;
        is_valid &= validation::validate_range(ctx, path, "red", &self.red, 0.0, 1.0, true);
        is_valid &= validation::validate_range(ctx, path, "green", &self.green, 0.0, 1.0, true);
        is_valid &= validation::validate_range(ctx, path, "blue", &self.blue, 0.0, 1.0, true);
        is_valid
    }
} */

define_three_component_color_space!(A98RGB, red, green, blue);

/* impl ValidateColorComponentValue for A98RGB {
    fn validate(&self, ctx: &mut ParserContext, path: &str) -> bool {
        let mut is_valid = true;
        is_valid &= validation::validate_range(ctx, path, "red", &self.red, 0.0, 1.0, true);
        is_valid &= validation::validate_range(ctx, path, "green", &self.green, 0.0, 1.0, true);
        is_valid &= validation::validate_range(ctx, path, "blue", &self.blue, 0.0, 1.0, true);
        is_valid
    }
} */

define_three_component_color_space!(ProPhotoRGB, red, green, blue);

/* impl ValidateColorComponentValue for ProPhotoRGB {
    fn validate(&self, ctx: &mut ParserContext, path: &str) -> bool {
        let mut is_valid = true;
        is_valid &= validation::validate_range(ctx, path, "red", &self.red, 0.0, 1.0, true);
        is_valid &= validation::validate_range(ctx, path, "green", &self.green, 0.0, 1.0, true);
        is_valid &= validation::validate_range(ctx, path, "blue", &self.blue, 0.0, 1.0, true);
        is_valid
    }
} */

define_three_component_color_space!(Rec2020, red, green, blue);

/* impl ValidateColorComponentValue for Rec2020 {
    fn validate(&self, ctx: &mut ParserContext, path: &str) -> bool {
        let mut is_valid = true;
        is_valid &= validation::validate_range(ctx, path, "red", &self.red, 0.0, 1.0, true);
        is_valid &= validation::validate_range(ctx, path, "green", &self.green, 0.0, 1.0, true);
        is_valid &= validation::validate_range(ctx, path, "blue", &self.blue, 0.0, 1.0, true);
        is_valid
    }
} */

define_three_component_color_space!(XYZD50, x, y, z);

/* impl ValidateColorComponentValue for XYZD50 {
    fn validate(&self, ctx: &mut ParserContext, path: &str) -> bool {
        let mut is_valid = true;
        is_valid &= validation::validate_range(ctx, path, "x", &self.x, 0.0, 1.0, true);
        is_valid &= validation::validate_range(ctx, path, "y", &self.y, 0.0, 1.0, true);
        is_valid &= validation::validate_range(ctx, path, "z", &self.z, 0.0, 1.0, true);
        is_valid
    }
} */

define_three_component_color_space!(XYZD65, x, y, z);

/* impl ValidateColorComponentValue for XYZD65 {
    fn validate(&self, ctx: &mut ParserContext, path: &str) -> bool {
        let mut is_valid = true;
        is_valid &= validation::validate_range(ctx, path, "x", &self.x, 0.0, 1.0, true);
        is_valid &= validation::validate_range(ctx, path, "y", &self.y, 0.0, 1.0, true);
        is_valid &= validation::validate_range(ctx, path, "z", &self.z, 0.0, 1.0, true);
        is_valid
    }
} */

/// The enum specifies all of the values the "components" key (which is the value of the color tokens) can be
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

/* impl ValidateColorComponentValue for ColorComponentValue {
    fn validate(&self, ctx: &mut ParserContext, path: &str) -> bool {
        match self {
            ColorComponentValue::SRGB(srgb) => srgb.validate(ctx, path),
            ColorComponentValue::SRGBLinear(srgb_linear) => srgb_linear.validate(ctx, path),
            ColorComponentValue::HSL(hsl) => hsl.validate(ctx, path),
            ColorComponentValue::HWB(hwb) => hwb.validate(ctx, path),
            ColorComponentValue::CIELAB(cielab) => cielab.validate(ctx, path),
            ColorComponentValue::LCH(lch) => lch.validate(ctx, path),
            ColorComponentValue::OKLAB(oklab) => oklab.validate(ctx, path),
            ColorComponentValue::OKLCH(oklch) => oklch.validate(ctx, path),
            ColorComponentValue::DisplayP3(display_p3) => display_p3.validate(ctx, path),
            ColorComponentValue::A98RGB(a98rgb) => a98rgb.validate(ctx, path),
            ColorComponentValue::ProPhotoRGB(prophoto_rgb) => prophoto_rgb.validate(ctx, path),
            ColorComponentValue::Rec2020(rec2020) => rec2020.validate(ctx, path),
            ColorComponentValue::XYZD65(xyz_d65) => xyz_d65.validate(ctx, path),
            ColorComponentValue::XYZD50(xyz_d50) => xyz_d50.validate(ctx, path),
        }
    }
} */

/// The ColorTokenValue struct represents the value of a color token, which includes the required color space and components of the color token,
/// as well as the optional alpha and hex values of the color token.
pub struct ColorTokenValue {
    /// The color space of the color token, which determines how the components of the color token should be interpreted
    pub color_space: ColorSpace,
    /// The components of the color token, which are interpreted according to the color space of the color token
    pub components: ColorComponentValue,
    /// The alpha component of the color token, which is a float between 0 and 1 representing the opacity of the color.
    /// If the alpha component is not specified, it is assumed to be 1 (fully opaque).
    pub alpha: Option<f64>,
    /// The hex value of the color token, which is a string representing the color in hexadecimal notation (e.g. "#RRGGBB").
    pub hex: Option<HexColorTokenValue>,
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
                let raw_color_space = require_object_field(ctx, path, map, "colorSpace");
                let raw_components = require_object_field(ctx, path, map, "components");
                let raw_alpha = map.get("alpha");
                let raw_hex = map.get("hex");

                // Parse the color space of the color token, which is a required string that specifies the color space of the color components
                // which is required to determine how the components of the color token should be interpreted
                let parsed_color_space = raw_color_space.and_then(|raw| {
                    require_enum_string_with_mapping(ctx, path, "colorSpace", raw, |s| {
                        match s {
                            "srgb" => Some(ColorSpace::SRGB),
                            "srgb-linear" => Some(ColorSpace::SRGBLinear),
                            "hsl" => Some(ColorSpace::HSL),
                            "hwb" => Some(ColorSpace::HWB),
                            "cielab" => Some(ColorSpace::CIELAB),
                            "lch" => Some(ColorSpace::LCH),
                            "oklab" => Some(ColorSpace::OKLAB),
                            "oklch" => Some(ColorSpace::OKLCH),
                            "display-p3" => Some(ColorSpace::DisplayP3),
                            "a98-rgb" => Some(ColorSpace::A98RGB),
                            "prophoto-rgb" => Some(ColorSpace::ProPhotoRGB),
                            "rec2020" => Some(ColorSpace::Rec2020),
                            "xyz-d65" => Some(ColorSpace::XYZD65),
                            "xyz-d50" => Some(ColorSpace::XYZD50),
                            _ => None,
                        }
                    }, "srgb, srgb-linear, hsl, hwb, cielab, lch, oklab, oklch, display-p3, a98-rgb, prophoto-rgb, rec2020, xyz-d65, xyz-d50")
                });

                // Parse the components of the color token. It is an array with a length of 3, and the interpretation of the components depends on the color space of the token
                /* let parsed_components = raw_components.and_then(|raw| {
                    require_array(ctx, path, raw)
                        .and_then(|array| validation::validate_component_array(ctx, path, array))
                        .and_then(|components| match parsed_color_space {
                            Some(ColorSpace::SRGB) => {
                                Some(ColorComponentValue::SRGB(SRGB::from(components)))
                            }
                            Some(ColorSpace::SRGBLinear) => Some(ColorComponentValue::SRGBLinear(
                                SRGBLinear::from(components),
                            )),
                            Some(ColorSpace::HSL) => {
                                Some(ColorComponentValue::HSL(HSL::from(components)))
                            }
                            Some(ColorSpace::HWB) => {
                                Some(ColorComponentValue::HWB(HWB::from(components)))
                            }
                            Some(ColorSpace::CIELAB) => {
                                Some(ColorComponentValue::CIELAB(CIELAB::from(components)))
                            }
                            Some(ColorSpace::LCH) => {
                                Some(ColorComponentValue::LCH(LCH::from(components)))
                            }
                            Some(ColorSpace::OKLAB) => {
                                Some(ColorComponentValue::OKLAB(OKLAB::from(components)))
                            }
                            Some(ColorSpace::OKLCH) => {
                                Some(ColorComponentValue::OKLCH(OKLCH::from(components)))
                            }
                            Some(ColorSpace::DisplayP3) => {
                                Some(ColorComponentValue::DisplayP3(DisplayP3::from(components)))
                            }
                            Some(ColorSpace::A98RGB) => {
                                Some(ColorComponentValue::A98RGB(A98RGB::from(components)))
                            }
                            Some(ColorSpace::ProPhotoRGB) => Some(
                                ColorComponentValue::ProPhotoRGB(ProPhotoRGB::from(components)),
                            ),
                            Some(ColorSpace::Rec2020) => {
                                Some(ColorComponentValue::Rec2020(Rec2020::from(components)))
                            }
                            Some(ColorSpace::XYZD65) => {
                                Some(ColorComponentValue::XYZD65(XYZD65::from(components)))
                            }
                            Some(ColorSpace::XYZD50) => {
                                Some(ColorComponentValue::XYZD50(XYZD50::from(components)))
                            }
                            None => None,
                        })
                        .and_then(|converted_value| {
                            if converted_value.validate(ctx, &path) {
                                Some(converted_value)
                            } else {
                                None
                            }
                        })
                }); */

                // Parse the alpha component of the color token, which is an optional number between 0 and 1 representing the opacity of the color. If the alpha component is not specified, it is assumed to be 1 (fully opaque)
                let parsed_alpha = raw_alpha.and_then(|raw| {
                    require_number(ctx, path, raw).and_then(|num| {
                        num.as_f64().and_then(|alpha| {
                            if validation::validate_alpha(ctx, path, Some(alpha)) {
                                Some(alpha)
                            } else {
                                // We won't error here as alpha is optional, but we will add a warning to the parser context if the alpha value is invalid
                                // which is what is in this branch
                                ctx.push_to_warnings(
                                    DiagnosticCode::InvalidPropertyValue,
                                    format!("Invalid alpha value: {}", alpha),
                                    path.to_string(),
                                );
                                None
                            }
                        })
                    })
                });

                // Parse the hex value of the color token, which is an optional string representing the color in hexadecimal notation (e.g. "#RRGGBB"). If the hex value is specified, it must be a valid hex color string and the color components of the color token must match the hex value of the color token
                let parsed_hex = raw_hex.and_then(|raw| {
                    require_string(ctx, path, raw).and_then(|s| {
                        if validation::validate_hex_color_token_value(&String::from(s)) {
                            Some(s.to_string())
                        } else {
                            // We won't error here as hex is optional, but we will add a warning to the parser context if the hex value is invalid
                            // which is what is in this branch
                            ctx.push_to_warnings(
                                DiagnosticCode::InvalidPropertyValue,
                                format!("Invalid hex value: {}", s),
                                path.to_string(),
                            );
                            None
                        }
                    })
                });

                // Finally, construct the ColorTokenValue struct if the required color space and components were successfully parsed, otherwise return an error
                match (parsed_color_space) {
                    (Some(color_space)) =>
                    /* ParseState::Parsed(Self {
                        color_space,
                        components,
                        alpha: parsed_alpha,
                        hex: parsed_hex,
                    }), */
                    {
                        ParseState::Skipped
                    }
                    _ => ParseState::Skipped,
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
    use crate::{FileFormat, errors::DiagnosticCode};
    use serde_json::json;

    fn make_context() -> ParserContext {
        ParserContext::new(String::from("test.json"), FileFormat::Json, String::new())
    }

    fn parse_color(value: &serde_json::Value) -> (ParseState<ColorTokenValue>, ParserContext) {
        let mut ctx = make_context();
        let result = ColorTokenValue::try_from_json(&mut ctx, "tokens.colors.primary", value);
        (result, ctx)
    }

    fn matches_color_space_and_components(value: &ColorTokenValue, color_space: &str) -> bool {
        match color_space {
            "srgb" => matches!(
                (&value.color_space, &value.components),
                (ColorSpace::SRGB, ColorComponentValue::SRGB(_))
            ),
            "srgb-linear" => matches!(
                (&value.color_space, &value.components),
                (ColorSpace::SRGBLinear, ColorComponentValue::SRGBLinear(_))
            ),
            "hsl" => matches!(
                (&value.color_space, &value.components),
                (ColorSpace::HSL, ColorComponentValue::HSL(_))
            ),
            "hwb" => matches!(
                (&value.color_space, &value.components),
                (ColorSpace::HWB, ColorComponentValue::HWB(_))
            ),
            "cielab" => matches!(
                (&value.color_space, &value.components),
                (ColorSpace::CIELAB, ColorComponentValue::CIELAB(_))
            ),
            "lch" => matches!(
                (&value.color_space, &value.components),
                (ColorSpace::LCH, ColorComponentValue::LCH(_))
            ),
            "oklab" => matches!(
                (&value.color_space, &value.components),
                (ColorSpace::OKLAB, ColorComponentValue::OKLAB(_))
            ),
            "oklch" => matches!(
                (&value.color_space, &value.components),
                (ColorSpace::OKLCH, ColorComponentValue::OKLCH(_))
            ),
            "display-p3" => matches!(
                (&value.color_space, &value.components),
                (ColorSpace::DisplayP3, ColorComponentValue::DisplayP3(_))
            ),
            "a98-rgb" => matches!(
                (&value.color_space, &value.components),
                (ColorSpace::A98RGB, ColorComponentValue::A98RGB(_))
            ),
            "prophoto-rgb" => matches!(
                (&value.color_space, &value.components),
                (ColorSpace::ProPhotoRGB, ColorComponentValue::ProPhotoRGB(_))
            ),
            "rec2020" => matches!(
                (&value.color_space, &value.components),
                (ColorSpace::Rec2020, ColorComponentValue::Rec2020(_))
            ),
            "xyz-d65" => matches!(
                (&value.color_space, &value.components),
                (ColorSpace::XYZD65, ColorComponentValue::XYZD65(_))
            ),
            "xyz-d50" => matches!(
                (&value.color_space, &value.components),
                (ColorSpace::XYZD50, ColorComponentValue::XYZD50(_))
            ),
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

        let ParseState::Parsed(parsed) = result else {
            panic!("expected color token to parse successfully");
        };

        assert!(matches!(parsed.color_space, ColorSpace::SRGB));
        assert!(matches!(parsed.components, ColorComponentValue::SRGB(_)));
        assert_eq!(parsed.alpha, Some(0.4));
        assert_eq!(parsed.hex.as_deref(), Some("#abc"));
    }

    #[test]
    fn parses_none_component_for_lch_color() {
        let value = json!({
            "colorSpace": "lch",
            "components": [50.0, "none", 180.0]
        });

        let (result, ctx) = parse_color(&value);

        assert!(ctx.errors.is_empty());

        let ParseState::Parsed(parsed) = result else {
            panic!("expected color token to parse successfully");
        };

        /* assert!(matches!(parsed.color_space, ColorSpace::LCH));
        match parsed.components {
            ColorComponentValue::LCH(lch) => {
                assert!(matches!(lch.lightness, ColorComponentElement::Value(50.0)));
                assert!(matches!(lch.chroma, ColorComponentElement::None));
                assert!(matches!(lch.hue, ColorComponentElement::Value(180.0)));
            }
            _ => panic!("expected LCH color components"),
        } */
    }

    #[test]
    fn parses_all_supported_color_spaces_with_valid_components() {
        let cases = [
            ("srgb", json!([0.0, 1.0, 0.5])),
            ("srgb-linear", json!([0.0, 1.0, 0.5])),
            ("hsl", json!([359.0, 0.0, 100.0])),
            ("hwb", json!([0.0, 0.0, 100.0])),
            ("cielab", json!([0.0, -50.0, 50.0])),
            ("lch", json!([100.0, 0.0, 359.0])),
            ("oklab", json!([50.0, -0.5, 0.5])),
            ("oklch", json!([100.0, 0.0, 359.0])),
            ("display-p3", json!([0.0, 1.0, 0.5])),
            ("a98-rgb", json!([0.0, 1.0, 0.5])),
            ("prophoto-rgb", json!([0.0, 1.0, 0.5])),
            ("rec2020", json!([0.0, 1.0, 0.5])),
            ("xyz-d65", json!([0.0, 1.0, 0.5])),
            ("xyz-d50", json!([0.0, 1.0, 0.5])),
        ];

        for (color_space, components) in cases {
            let value = json!({
                "colorSpace": color_space,
                "components": components
            });

            let (result, ctx) = parse_color(&value);

            assert!(
                ctx.errors.is_empty(),
                "unexpected diagnostics for {color_space}"
            );

            let ParseState::Parsed(parsed) = result else {
                panic!("expected {color_space} color token to parse successfully");
            };

            assert!(
                matches_color_space_and_components(&parsed, color_space),
                "expected color space and components to match for {color_space}"
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
            "Expected one of srgb, srgb-linear, hsl, hwb, cielab, lch, oklab, oklch, display-p3, a98-rgb, prophoto-rgb, rec2020, xyz-d65, xyz-d50 for the field 'colorSpace', but got 'ansi-rgb'"
        );
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
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyType);
        assert_eq!(ctx.errors[0].message, "Expected an array, but found: true");
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
    }

    #[test]
    fn skips_when_component_keyword_is_not_none() {
        let value = json!({
            "colorSpace": "srgb",
            "components": [0.1, "invalid", 0.3]
        });

        let (result, ctx) = parse_color(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(
            ctx.errors[0].message,
            "Expected a string with the value 'none' or a number for color components, but got a string with the value 'invalid'"
        );
    }

    #[test]
    fn skips_when_component_values_fail_color_space_validation() {
        let value = json!({
            "colorSpace": "hsl",
            "components": [360.0, 50.0, 50.0]
        });

        let (result, ctx) = parse_color(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(
            ctx.errors[0].message,
            "Invalid color component value at path 'tokens.colors.primary.hue': Value must be between 0 and 360 (exclusive)"
        );
    }

    #[test]
    fn keeps_parsing_when_alpha_is_invalid_but_records_diagnostic() {
        let value = json!({
            "colorSpace": "srgb",
            "components": [0.1, 0.2, 0.3],
            "alpha": 1.5
        });

        let (result, ctx) = parse_color(&value);

        let ParseState::Parsed(parsed) = result else {
            panic!("expected color token to parse despite invalid alpha");
        };

        assert_eq!(parsed.alpha, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(
            ctx.errors[0].message,
            "Invalid color component value at path 'tokens.colors.primary.alpha': Alpha value must be between 0 and 1 (inclusive)"
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

        let ParseState::Parsed(parsed) = result else {
            panic!("expected color token to parse despite invalid alpha type");
        };

        assert_eq!(parsed.alpha, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyType);
        assert_eq!(
            ctx.errors[0].message,
            "Expected a number, but found: \"opaque\""
        );
    }

    #[test]
    fn keeps_parsing_when_hex_is_invalid_or_wrong_type() {
        let invalid_hex_value = json!({
            "colorSpace": "srgb",
            "components": [0.1, 0.2, 0.3],
            "hex": "#12"
        });

        let (invalid_hex_result, invalid_hex_ctx) = parse_color(&invalid_hex_value);

        let ParseState::Parsed(parsed_invalid_hex) = invalid_hex_result else {
            panic!("expected color token to parse despite invalid hex contents");
        };

        assert_eq!(parsed_invalid_hex.hex, None);
        assert!(invalid_hex_ctx.errors.is_empty());

        let wrong_type_hex_value = json!({
            "colorSpace": "srgb",
            "components": [0.1, 0.2, 0.3],
            "hex": false
        });

        let (wrong_type_result, wrong_type_ctx) = parse_color(&wrong_type_hex_value);

        let ParseState::Parsed(parsed_wrong_type_hex) = wrong_type_result else {
            panic!("expected color token to parse despite invalid hex type");
        };

        assert_eq!(parsed_wrong_type_hex.hex, None);
        assert_eq!(wrong_type_ctx.errors.len(), 1);
        assert_eq!(
            wrong_type_ctx.errors[0].code,
            DiagnosticCode::InvalidPropertyType
        );
        assert_eq!(
            wrong_type_ctx.errors[0].message,
            "Expected a string, but found: false"
        );
    }

    #[test]
    fn skips_when_lch_chroma_is_negative() {
        let value = json!({
            "colorSpace": "lch",
            "components": [50.0, -1.0, 180.0]
        });

        let (result, ctx) = parse_color(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(
            ctx.errors[0].message,
            "Invalid color component value at path 'tokens.colors.primary.chroma': Value must be non-negative"
        );
    }

    #[test]
    fn skips_when_xyz_component_exceeds_maximum() {
        let value = json!({
            "colorSpace": "xyz-d65",
            "components": [0.1, 1.1, 0.3]
        });

        let (result, ctx) = parse_color(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(
            ctx.errors[0].message,
            "Invalid color component value at path 'tokens.colors.primary.y': Value must be between 0 and 1 (inclusive)"
        );
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
