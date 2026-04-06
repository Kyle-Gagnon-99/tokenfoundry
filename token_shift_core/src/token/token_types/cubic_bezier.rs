//! The `cubic_bezier` module defines the data structures for cubic bezier tokens defined in the DTCG specification,
//! which represents a cubic bezier curve in the UI, such as the easing function of an animation or transition.

use crate::{
    ParserContext,
    errors::DiagnosticCode,
    token::{ParseState, TryFromJson},
};

/// Represents a cubic bezier token value, which consists of four numeric values representing the control points of the cubic bezier curve
/// The DTCG specification defines that a cubic bezier token value shall have "x1", "y1", "x2", and "y2" properties, which are all numbers
/// The first control point is represented by (x1, y1) and the second control point is represented by (x2, y2)
/// The y coordinates of P1 and P2 can be any real number, but the x coordinates of P1 and P2 must be in the range [0, 1], inclusive
pub struct CubicBezierTokenValue {
    pub x1: f64,
    pub y1: f64,
    pub x2: f64,
    pub y2: f64,
}

impl CubicBezierTokenValue {
    /// Validates the cubic bezier token value according to the DTCG specification
    ///
    /// # Returns
    ///
    /// Returns true if the value is valid, and false otherwise
    pub fn validate(&self, ctx: &mut ParserContext, path: &str) -> bool {
        let mut is_valid = true;

        if self.x1 < 0.0 || self.x1 > 1.0 {
            ctx.push_to_errors(
                DiagnosticCode::InvalidTokenValue,
                format!(
                    "Expected x1 of cubic bezier token value to be in the range [0, 1], but found {}",
                    self.x1
                ),
                format!("{}.$value.[0]", path),
            );
            is_valid = false;
        }

        if self.x2 < 0.0 || self.x2 > 1.0 {
            ctx.push_to_errors(
                DiagnosticCode::InvalidTokenValue,
                format!(
                    "Expected x2 of cubic bezier token value to be in the range [0, 1], but found {}",
                    self.x2
                ),
                format!("{}.$value.[2]", path),
            );
            is_valid = false;
        }

        is_valid
    }
}

pub struct CubicBezierToken {
    pub value: CubicBezierTokenValue,
}

impl<'a> TryFromJson<'a> for CubicBezierToken {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> crate::token::ParseState<Self> {
        match value {
            serde_json::Value::Array(array) => {
                // First, we need to check if the array has exactly 4 items
                if array.len() != 4 {
                    ctx.push_to_errors(
                        DiagnosticCode::InvalidTokenValue,
                        format!(
                            "Expected cubic bezier token value to be an array of 4 numbers, but found an array of length {}",
                            array.len()
                        ),
                        path.into(),
                    );
                    return ParseState::Skipped;
                }

                // Then, we need to check if all items in the array are numbers and parse them as f64
                let mut values = [0.0; 4];
                for (index, item) in array.iter().enumerate() {
                    match item {
                        serde_json::Value::Number(num) => {
                            if let Some(parsed) = num.as_f64() {
                                values[index] = parsed;
                            } else {
                                ctx.push_to_errors(
                                    DiagnosticCode::InvalidTokenValue,
                                    format!(
                                        "Expected cubic bezier token value to be an array of numbers, but found a number that cannot be parsed as f64: {}",
                                        num
                                    ),
                                    format!("{}.{}", path, index),
                                );
                                return ParseState::Skipped;
                            }
                        }
                        _ => {
                            ctx.push_to_errors(
                                DiagnosticCode::InvalidTokenValue,
                                format!(
                                    "Expected cubic bezier token value to be an array of numbers, but found a non-number value: {}",
                                    item
                                ),
                                format!("{}.{}", path, index),
                            );
                            return ParseState::Skipped;
                        }
                    }
                }

                // Finally, we can construct the CubicBezierTokenValue and validate it
                let token_value = CubicBezierTokenValue {
                    x1: values[0],
                    y1: values[1],
                    x2: values[2],
                    y2: values[3],
                };

                if !token_value.validate(ctx, path) {
                    return ParseState::Skipped;
                }

                ParseState::Parsed(CubicBezierToken { value: token_value })
            }
            _ => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidTokenValue,
                    format!(
                        "Expected cubic bezier token value to be an array of 4 numbers, but found {}",
                        value
                    ),
                    path.into(),
                );
                ParseState::Skipped
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{FileFormat, ParserContext, errors::DiagnosticCode};
    use serde_json::json;

    fn make_context() -> ParserContext {
        ParserContext::new(String::from("test.json"), FileFormat::Json, String::new())
    }

    fn parse_cubic_bezier(
        value: &serde_json::Value,
    ) -> (ParseState<CubicBezierToken>, ParserContext) {
        let mut ctx = make_context();
        let result = CubicBezierToken::try_from_json(&mut ctx, "tokens.motion.ease", value);
        (result, ctx)
    }

    #[test]
    fn parses_valid_cubic_bezier_array() {
        let value = json!([0.25, 0.1, 0.25, 1.0]);

        let (result, ctx) = parse_cubic_bezier(&value);

        assert!(ctx.errors.is_empty());

        let ParseState::Parsed(parsed) = result else {
            panic!("expected cubic bezier token to parse successfully");
        };

        assert_eq!(parsed.value.x1, 0.25);
        assert_eq!(parsed.value.y1, 0.1);
        assert_eq!(parsed.value.x2, 0.25);
        assert_eq!(parsed.value.y2, 1.0);
    }

    #[test]
    fn parses_boundary_x_values() {
        let value = json!([0.0, -2.0, 1.0, 2.0]);

        let (result, ctx) = parse_cubic_bezier(&value);

        assert!(ctx.errors.is_empty());

        let ParseState::Parsed(parsed) = result else {
            panic!("expected cubic bezier token to parse successfully");
        };

        assert_eq!(parsed.value.x1, 0.0);
        assert_eq!(parsed.value.x2, 1.0);
    }

    #[test]
    fn skips_when_array_length_is_not_four() {
        let value = json!([0.25, 0.1, 0.25]);

        let (result, ctx) = parse_cubic_bezier(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(
            ctx.errors[0].message,
            "Expected cubic bezier token value to be an array of 4 numbers, but found an array of length 3"
        );
        assert_eq!(ctx.errors[0].path, "tokens.motion.ease");
    }

    #[test]
    fn skips_when_array_contains_non_number_value() {
        let value = json!([0.25, "bad", 0.25, 1.0]);

        let (result, ctx) = parse_cubic_bezier(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(
            ctx.errors[0].message,
            "Expected cubic bezier token value to be an array of numbers, but found a non-number value: \"bad\""
        );
        assert_eq!(ctx.errors[0].path, "tokens.motion.ease.1");
    }

    #[test]
    fn skips_for_non_array_input() {
        let value = json!("ease-in-out");

        let (result, ctx) = parse_cubic_bezier(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(
            ctx.errors[0].message,
            "Expected cubic bezier token value to be an array of 4 numbers, but found \"ease-in-out\""
        );
        assert_eq!(ctx.errors[0].path, "tokens.motion.ease");
    }

    #[test]
    fn validate_accepts_in_range_x_values() {
        let mut ctx = make_context();
        let value = CubicBezierTokenValue {
            x1: 0.0,
            y1: 10.0,
            x2: 1.0,
            y2: -10.0,
        };

        let is_valid = value.validate(&mut ctx, "tokens.motion.ease");

        assert!(is_valid);
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn validate_rejects_out_of_range_x_values_and_collects_both_errors() {
        let mut ctx = make_context();
        let value = CubicBezierTokenValue {
            x1: -0.1,
            y1: 0.0,
            x2: 1.1,
            y2: 1.0,
        };

        let is_valid = value.validate(&mut ctx, "tokens.motion.ease");

        assert!(!is_valid);
        assert_eq!(ctx.errors.len(), 2);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(
            ctx.errors[0].message,
            "Expected x1 of cubic bezier token value to be in the range [0, 1], but found -0.1"
        );
        assert_eq!(ctx.errors[0].path, "tokens.motion.ease.$value.[0]");
        assert_eq!(ctx.errors[1].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(
            ctx.errors[1].message,
            "Expected x2 of cubic bezier token value to be in the range [0, 1], but found 1.1"
        );
        assert_eq!(ctx.errors[1].path, "tokens.motion.ease.$value.[2]");
    }

    #[test]
    fn parser_skips_when_validation_fails() {
        let value = json!([-0.1, 0.0, 1.1, 1.0]);

        let (result, ctx) = parse_cubic_bezier(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 2);
        assert_eq!(ctx.errors[0].path, "tokens.motion.ease.$value.[0]");
        assert_eq!(ctx.errors[1].path, "tokens.motion.ease.$value.[2]");
    }
}
