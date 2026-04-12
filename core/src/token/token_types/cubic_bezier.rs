//! The `cubic_bezier` module defines the data structures for cubic bezier tokens defined in the DTCG specification,
//! which represents a cubic bezier curve in the UI, such as the easing function of an animation or transition.

use crate::{
    ParserContext,
    errors::DiagnosticCode,
    ir::{JsonNumber, RefOr, TryFromJson, parse_ref_or_value},
};

/// Represents a cubic bezier token value, which consists of four numeric values representing the control points of the cubic bezier curve
/// The DTCG specification defines that a cubic bezier token value shall have "x1", "y1", "x2", and "y2" properties, which are all numbers
/// The first control point is represented by (x1, y1) and the second control point is represented by (x2, y2)
/// The y coordinates of P1 and P2 can be any real number, but the x coordinates of P1 and P2 must be in the range [0, 1], inclusive
#[derive(Debug, Clone, PartialEq)]
pub struct CubicBezierValue([RefOr<JsonNumber>; 4]);

impl<'a> TryFromJson<'a> for CubicBezierValue {
    fn try_from_json(
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

                let mut values: [RefOr<JsonNumber>; 4] = core::array::from_fn(|_| {
                    RefOr::Literal(JsonNumber(serde_json::Number::from_f64(0.0).unwrap()))
                });
                for (index, item) in arr_val.iter().enumerate() {
                    let parsed_val = parse_ref_or_value::<JsonNumber>(ctx, path, item);
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
pub struct CubicBezierTokenValue(pub RefOr<CubicBezierValue>);

impl<'a> TryFromJson<'a> for CubicBezierTokenValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        parse_ref_or_value(ctx, path, value).map(CubicBezierTokenValue)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        FileFormat, ParserContext,
        errors::DiagnosticCode,
        ir::{JsonLocalPointer, JsonRef, RefOr},
    };
    use serde_json::{Number, json};

    fn parser_context() -> ParserContext {
        ParserContext::new("tests.json".into(), FileFormat::Json, String::new())
    }

    #[test]
    fn parses_cubic_bezier_literal_array() {
        let mut ctx = parser_context();

        let parsed = CubicBezierTokenValue::try_from_json(
            &mut ctx,
            "/token",
            &json!([0.25, 0.1, 0.25, 1.0]),
        );

        assert_eq!(
            parsed,
            Some(CubicBezierTokenValue(RefOr::Literal(CubicBezierValue([
                RefOr::Literal(JsonNumber(Number::from_f64(0.25).unwrap())),
                RefOr::Literal(JsonNumber(Number::from_f64(0.1).unwrap())),
                RefOr::Literal(JsonNumber(Number::from_f64(0.25).unwrap())),
                RefOr::Literal(JsonNumber(Number::from_f64(1.0).unwrap())),
            ]))))
        );
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn parses_cubic_bezier_with_mixed_literal_and_reference_values() {
        let mut ctx = parser_context();
        let value = json!([
            0.42,
            { "$ref": "#/motion/easing/y1" },
            1.0,
            { "$ref": "#/motion/easing/y2" }
        ]);

        let parsed = CubicBezierTokenValue::try_from_json(&mut ctx, "/token", &value);

        assert_eq!(
            parsed,
            Some(CubicBezierTokenValue(RefOr::Literal(CubicBezierValue([
                RefOr::Literal(JsonNumber(Number::from_f64(0.42).unwrap())),
                RefOr::Ref(JsonRef::new_local_pointer(
                    "#/motion/easing/y1".into(),
                    crate::ir::JsonPointer::from("#/motion/easing/y1"),
                )),
                RefOr::Literal(JsonNumber(Number::from_f64(1.0).unwrap())),
                RefOr::Ref(JsonRef::new_local_pointer(
                    "#/motion/easing/y2".into(),
                    crate::ir::JsonPointer::from("#/motion/easing/y2"),
                )),
            ]))))
        );
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn rejects_cubic_bezier_when_array_is_too_short() {
        let mut ctx = parser_context();

        let parsed =
            CubicBezierTokenValue::try_from_json(&mut ctx, "/token", &json!([0.25, 0.1, 0.25]));

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyValue);
        assert_eq!(ctx.errors[0].path, "/token");
    }

    #[test]
    fn rejects_cubic_bezier_when_array_is_too_long() {
        let mut ctx = parser_context();

        let parsed = CubicBezierTokenValue::try_from_json(
            &mut ctx,
            "/token",
            &json!([0.25, 0.1, 0.25, 1.0, 2.0]),
        );

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyValue);
        assert_eq!(ctx.errors[0].path, "/token");
    }

    #[test]
    fn rejects_cubic_bezier_when_top_level_value_is_not_an_array() {
        let mut ctx = parser_context();

        let parsed = CubicBezierTokenValue::try_from_json(
            &mut ctx,
            "/token",
            &json!({ "x1": 0.25, "y1": 0.1, "x2": 0.25, "y2": 1.0 }),
        );

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyValue);
        assert_eq!(ctx.errors[0].path, "/token");
    }

    #[test]
    fn rejects_cubic_bezier_when_item_has_invalid_type() {
        let mut ctx = parser_context();

        let parsed = CubicBezierTokenValue::try_from_json(
            &mut ctx,
            "/token",
            &json!([0.25, "bad", 0.25, 1.0]),
        );

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 2);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyType);
        assert_eq!(ctx.errors[0].path, "/token");
        assert_eq!(ctx.errors[1].code, DiagnosticCode::InvalidPropertyValue);
        assert_eq!(ctx.errors[1].path, "/token.1");
    }

    #[test]
    fn rejects_cubic_bezier_when_item_reference_is_invalid() {
        let mut ctx = parser_context();

        let parsed = CubicBezierTokenValue::try_from_json(
            &mut ctx,
            "/token",
            &json!([0.25, { "$ref": "not-a-pointer" }, 0.25, 1.0]),
        );

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 2);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidReference);
        assert_eq!(ctx.errors[0].path, "/token");
        assert_eq!(ctx.errors[1].code, DiagnosticCode::InvalidPropertyValue);
        assert_eq!(ctx.errors[1].path, "/token.1");
    }

    #[test]
    fn parses_cubic_bezier_token_as_literal_value() {
        let mut ctx = parser_context();

        let parsed = CubicBezierTokenValue::try_from_json(
            &mut ctx,
            "/token",
            &json!([0.42, 0.0, 0.58, 1.0]),
        );

        match parsed {
            Some(CubicBezierTokenValue(RefOr::Literal(CubicBezierValue(values)))) => {
                assert_eq!(
                    values,
                    [
                        RefOr::Literal(JsonNumber(Number::from_f64(0.42).unwrap())),
                        RefOr::Literal(JsonNumber(Number::from_f64(0.0).unwrap())),
                        RefOr::Literal(JsonNumber(Number::from_f64(0.58).unwrap())),
                        RefOr::Literal(JsonNumber(Number::from_f64(1.0).unwrap())),
                    ]
                );
            }
            _ => panic!("expected literal cubic bezier token"),
        }
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn parses_cubic_bezier_token_as_top_level_reference() {
        let mut ctx = parser_context();

        let parsed = CubicBezierTokenValue::try_from_json(
            &mut ctx,
            "/token",
            &json!({ "$ref": "#/motion/easing/standard" }),
        );

        match parsed {
            Some(CubicBezierTokenValue(RefOr::Ref(JsonRef::LocalPointer(JsonLocalPointer {
                raw_value,
                ..
            })))) => {
                assert_eq!(raw_value, "#/motion/easing/standard");
            }
            _ => panic!("expected top-level cubic bezier reference"),
        }
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn rejects_cubic_bezier_token_with_invalid_top_level_reference() {
        let mut ctx = parser_context();

        let parsed = CubicBezierTokenValue::try_from_json(
            &mut ctx,
            "/token",
            &json!({ "$ref": "not-a-pointer" }),
        );

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidReference);
        assert_eq!(ctx.errors[0].path, "/token");
    }
}
