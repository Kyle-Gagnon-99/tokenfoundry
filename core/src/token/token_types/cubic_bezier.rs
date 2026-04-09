//! The `cubic_bezier` module defines the data structures for cubic bezier tokens defined in the DTCG specification,
//! which represents a cubic bezier curve in the UI, such as the easing function of an animation or transition.

use crate::{
    ParserContext,
    errors::DiagnosticCode,
    ir::{RefOr, parse_ref_or_value},
    token::{ParseState, TryFromJson, TryFromJsonField, utils::FloatOrInteger},
};

/// Represents a cubic bezier token value, which consists of four numeric values representing the control points of the cubic bezier curve
/// The DTCG specification defines that a cubic bezier token value shall have "x1", "y1", "x2", and "y2" properties, which are all numbers
/// The first control point is represented by (x1, y1) and the second control point is represented by (x2, y2)
/// The y coordinates of P1 and P2 can be any real number, but the x coordinates of P1 and P2 must be in the range [0, 1], inclusive
#[derive(Debug, Clone, PartialEq)]
pub struct CubicBezierTokenValue([RefOr<FloatOrInteger>; 4]);

impl CubicBezierTokenValue {
    pub fn get_x1(&self) -> Option<f64> {
        match self.0[0] {
            RefOr::Literal(FloatOrInteger::Float(value)) => Some(value),
            RefOr::Literal(FloatOrInteger::Integer(value)) => Some(value as f64),
            RefOr::Ref(_) => None,
        }
    }

    pub fn get_y1(&self) -> Option<f64> {
        match self.0[1] {
            RefOr::Literal(FloatOrInteger::Float(value)) => Some(value),
            RefOr::Literal(FloatOrInteger::Integer(value)) => Some(value as f64),
            RefOr::Ref(_) => None,
        }
    }

    pub fn get_x2(&self) -> Option<f64> {
        match self.0[2] {
            RefOr::Literal(FloatOrInteger::Float(value)) => Some(value),
            RefOr::Literal(FloatOrInteger::Integer(value)) => Some(value as f64),
            RefOr::Ref(_) => None,
        }
    }

    pub fn get_y2(&self) -> Option<f64> {
        match self.0[3] {
            RefOr::Literal(FloatOrInteger::Float(value)) => Some(value),
            RefOr::Literal(FloatOrInteger::Integer(value)) => Some(value as f64),
            RefOr::Ref(_) => None,
        }
    }
}

impl<'a> TryFromJsonField<'a> for CubicBezierTokenValue {
    fn try_from_json_field(
        ctx: &mut ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        match value {
            serde_json::Value::Array(arr_val) => {
                if arr_val.len() != 4 {
                    ctx.push_to_errors(
                        DiagnosticCode::InvalidPropertyValue,
                        format!(
                            "Expected an array of 4 numbers, but found an array of length {}",
                            arr_val.len()
                        ),
                        path.into(),
                    );
                    return None;
                }

                let mut values: [RefOr<FloatOrInteger>; 4] =
                    core::array::from_fn(|_| RefOr::Literal(FloatOrInteger::Integer(0)));
                for (index, item) in arr_val.iter().enumerate() {
                    let parsed_val = parse_ref_or_value::<FloatOrInteger>(ctx, path, item);
                    match parsed_val {
                        Some(parsed) => values[index] = parsed,
                        None => {
                            ctx.push_to_errors(
                                DiagnosticCode::InvalidPropertyValue,
                                format!(
                                    "Expected a number or reference to a number, but found {}",
                                    item
                                ),
                                format!("{}.{}", path, index),
                            );
                            return None;
                        }
                    }
                }

                Some(Self(values))
            }
            _ => {
                ctx.push_to_errors(
                    DiagnosticCode::InvalidPropertyValue,
                    format!("Expected an array of 4 numbers, but found {}", value),
                    path.into(),
                );
                None
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CubicBezierToken(pub RefOr<CubicBezierTokenValue>);

impl<'a> TryFromJson<'a> for CubicBezierToken {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> crate::token::ParseState<Self> {
        match parse_ref_or_value::<CubicBezierTokenValue>(ctx, path, value) {
            Some(parsed_value) => ParseState::Parsed(Self(parsed_value)),
            None => ParseState::Skipped,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        FileFormat, ParserContext,
        errors::DiagnosticCode,
        ir::{JsonPointer, JsonRef, RefOr},
    };
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

    fn expect_parsed(result: ParseState<CubicBezierToken>) -> RefOr<CubicBezierTokenValue> {
        let ParseState::Parsed(CubicBezierToken(parsed)) = result else {
            panic!("expected cubic bezier token to parse successfully");
        };

        parsed
    }

    #[test]
    fn parses_literal_array_with_integer_and_float_values() {
        let value = json!([0.25, 0, 1, 1.0]);

        let (result, ctx) = parse_cubic_bezier(&value);

        assert!(ctx.errors.is_empty());

        assert_eq!(
            expect_parsed(result),
            RefOr::Literal(CubicBezierTokenValue([
                RefOr::Literal(FloatOrInteger::Float(0.25)),
                RefOr::Literal(FloatOrInteger::Integer(0)),
                RefOr::Literal(FloatOrInteger::Integer(1)),
                RefOr::Literal(FloatOrInteger::Float(1.0)),
            ]))
        );
    }

    #[test]
    fn parses_array_with_literal_and_reference_values() {
        let value = json!([
            0.0,
            { "$ref": "#/tokens/motion/shared/y1" },
            { "$ref": "" },
            -2.5
        ]);

        let (result, ctx) = parse_cubic_bezier(&value);

        assert!(ctx.errors.is_empty());

        let parsed = expect_parsed(result);

        assert_eq!(
            parsed,
            RefOr::Literal(CubicBezierTokenValue([
                RefOr::Literal(FloatOrInteger::Float(0.0)),
                RefOr::Ref(JsonRef::new_local_pointer(
                    "#/tokens/motion/shared/y1".to_string(),
                    JsonPointer::from("#/tokens/motion/shared/y1"),
                )),
                RefOr::Ref(JsonRef::new_local_pointer(
                    String::new(),
                    JsonPointer::new(),
                )),
                RefOr::Literal(FloatOrInteger::Float(-2.5)),
            ]))
        );

        let RefOr::Literal(parsed_value) = parsed else {
            panic!("expected a literal cubic bezier value");
        };

        assert_eq!(parsed_value.get_x1(), Some(0.0));
        assert_eq!(parsed_value.get_y1(), None);
        assert_eq!(parsed_value.get_x2(), None);
        assert_eq!(parsed_value.get_y2(), Some(-2.5));
    }

    #[test]
    fn parses_top_level_ref_object() {
        let value = json!({ "$ref": "#/tokens/motion/shared/ease" });

        let (result, ctx) = parse_cubic_bezier(&value);

        assert!(ctx.errors.is_empty());
        assert_eq!(
            expect_parsed(result),
            RefOr::Ref(JsonRef::new_local_pointer(
                "#/tokens/motion/shared/ease".to_string(),
                JsonPointer::from("#/tokens/motion/shared/ease"),
            ))
        );
    }

    #[test]
    fn skips_when_array_length_is_not_four() {
        let value = json!([0.25, 0.1, 0.25]);

        let (result, ctx) = parse_cubic_bezier(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyValue);
        assert_eq!(
            ctx.errors[0].message,
            "Expected an array of 4 numbers, but found an array of length 3"
        );
        assert_eq!(ctx.errors[0].path, "tokens.motion.ease");
    }

    #[test]
    fn skips_when_array_contains_non_number_value_and_reports_both_diagnostics() {
        let value = json!([0.25, "bad", 0.25, 1.0]);

        let (result, ctx) = parse_cubic_bezier(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 2);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyType);
        assert_eq!(
            ctx.errors[0].message,
            "Expected a number, but found: \"bad\""
        );
        assert_eq!(ctx.errors[0].path, "tokens.motion.ease");
        assert_eq!(ctx.errors[1].code, DiagnosticCode::InvalidPropertyValue);
        assert_eq!(
            ctx.errors[1].message,
            "Expected a number or reference to a number, but found \"bad\""
        );
        assert_eq!(ctx.errors[1].path, "tokens.motion.ease.1");
    }

    #[test]
    fn skips_when_array_contains_invalid_reference_and_reports_both_diagnostics() {
        let value = json!([0.25, { "$ref": "tokens/motion/shared/y1" }, 0.25, 1.0]);

        let (result, ctx) = parse_cubic_bezier(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 2);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidReference);
        assert_eq!(
            ctx.errors[0].message,
            "Invalid JSON pointer: tokens/motion/shared/y1"
        );
        assert_eq!(ctx.errors[0].path, "tokens.motion.ease");
        assert_eq!(ctx.errors[1].code, DiagnosticCode::InvalidPropertyValue);
        assert_eq!(
            ctx.errors[1].message,
            "Expected a number or reference to a number, but found {\"$ref\":\"tokens/motion/shared/y1\"}"
        );
        assert_eq!(ctx.errors[1].path, "tokens.motion.ease.1");
    }

    #[test]
    fn skips_for_non_array_input() {
        let value = json!("ease-in-out");

        let (result, ctx) = parse_cubic_bezier(&value);

        assert!(matches!(result, ParseState::Skipped));
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyValue);
        assert_eq!(
            ctx.errors[0].message,
            "Expected an array of 4 numbers, but found \"ease-in-out\""
        );
        assert_eq!(ctx.errors[0].path, "tokens.motion.ease");
    }
}
