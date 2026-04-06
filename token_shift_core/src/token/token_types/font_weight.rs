//! The `font_weight` module defines the data structures for font weight tokens defined in the DTCG specification,
//! which represents the weight of a font in the UI, such as normal, bold, or numeric values like 400, 700, etc.

use crate::token::TryFromJson;

/// The `FontWeightValueString` enum represents the allowed string values for font weight tokens, as defined in the DTCG specification.
/// This uses the OpenType wght tag specifcation for font weight strings
pub enum FontWeightValueString {
    Thin,
    Hairline,
    ExtraLight,
    UltraLight,
    Light,
    Normal,
    Regular,
    Book,
    Medium,
    SemiBold,
    DemiBold,
    Bold,
    ExtraBold,
    UltraBold,
    Black,
    Heavy,
    ExtraBlack,
    UltraBlack,
}

/// The `FromStr` implementation for `FontWeightValueString` allows parsing a string into a `FontWeightValueString` enum variant, while ignoring case and allowing for common synonyms as defined in the OpenType specification for font weight strings.
/// If the input string does not match any of the allowed font weight strings, it returns an error (in this case, an empty tuple `()`, but more details will be provided later).
impl std::str::FromStr for FontWeightValueString {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "thin" => Ok(FontWeightValueString::Thin),
            "hairline" => Ok(FontWeightValueString::Hairline),
            "extra-light" | "ultra-light" => Ok(FontWeightValueString::ExtraLight),
            "light" => Ok(FontWeightValueString::Light),
            "normal" | "regular" | "book" => Ok(FontWeightValueString::Normal),
            "medium" => Ok(FontWeightValueString::Medium),
            "semi-bold" | "demi-bold" => Ok(FontWeightValueString::SemiBold),
            "bold" => Ok(FontWeightValueString::Bold),
            "extra-bold" | "ultra-bold" => Ok(FontWeightValueString::ExtraBold),
            "black" | "heavy" => Ok(FontWeightValueString::Black),
            "extra-black" | "ultra-black" => Ok(FontWeightValueString::ExtraBlack),
            _ => Err(()),
        }
    }
}

/// The `FontWeightValue` enum represents the value of a font weight token, which can be either a numeric value (e.g. 400, 700) or a string value (e.g. "normal", "bold")
pub enum FontWeightValue {
    Numeric(u16),
    String(FontWeightValueString),
}

pub struct FontWeightTokenValue {
    pub value: FontWeightValue,
}

impl<'a> TryFromJson<'a> for FontWeightTokenValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> crate::token::ParseState<Self> {
        match value {
            serde_json::Value::Number(num) => {
                if let Some(u) = num.as_u64() {
                    if u >= 1 && u <= 1000 {
                        return crate::token::ParseState::Parsed(FontWeightTokenValue {
                            value: FontWeightValue::Numeric(u as u16),
                        });
                    }
                }
                ctx.push_to_errors(
                    crate::errors::DiagnosticCode::InvalidTokenValue,
                    format!(
                        "Expected font weight token numeric value to be an integer between 1 and 1000, but found {}",
                        num
                    ),
                    path.into(),
                );
                crate::token::ParseState::Skipped
            }
            serde_json::Value::String(str_value) => {
                match str_value.parse::<FontWeightValueString>() {
                    Ok(parsed_string) => crate::token::ParseState::Parsed(FontWeightTokenValue {
                        value: FontWeightValue::String(parsed_string),
                    }),
                    Err(_) => {
                        ctx.push_to_errors(
                            crate::errors::DiagnosticCode::InvalidTokenValue,
                            format!(
                                "Invalid font weight token string value '{}', expected one of the following: thin, hairline, extra-light, ultra-light, light, normal, regular, book, medium, semi-bold, demi-bold, bold, extra-bold, ultra-bold, black, heavy, extra-black, ultra-black",
                                str_value
                            ),
                            path.into(),
                        );
                        crate::token::ParseState::Skipped
                    }
                }
            }
            _ => {
                ctx.push_to_errors(
                    crate::errors::DiagnosticCode::InvalidTokenValue,
                    format!(
                        "Expected font weight token value to be either a number or a string, but found {}",
                        value
                    ),
                    path.into(),
                );
                crate::token::ParseState::Skipped
            }
        }
    }
}
