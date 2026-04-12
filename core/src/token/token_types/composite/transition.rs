//! The `transition` module defines the data structures for the transition token type, which is a composite token type
//! that represents an animated transition between two states.

use crate::{
    errors::DiagnosticCode,
    ir::{RefOr, TryFromJson, require_object},
    token::token_types::{
        composite::AliasOrLiteral, cubic_bezier::CubicBezierTokenValue,
        duration::DurationTokenValue,
    },
};

#[derive(Debug, Clone, PartialEq)]
pub struct TransitionDuration(pub AliasOrLiteral<DurationTokenValue>);

impl<'a> TryFromJson<'a> for TransitionDuration {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        AliasOrLiteral::try_from_json(ctx, path, value).map(TransitionDuration)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransitionDelay(pub AliasOrLiteral<DurationTokenValue>);

impl<'a> TryFromJson<'a> for TransitionDelay {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        AliasOrLiteral::try_from_json(ctx, path, value).map(TransitionDelay)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransitionTimingFunction(pub AliasOrLiteral<CubicBezierTokenValue>);

impl<'a> TryFromJson<'a> for TransitionTimingFunction {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        AliasOrLiteral::try_from_json(ctx, path, value).map(TransitionTimingFunction)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TransitionTokenValue {
    pub duration: RefOr<TransitionDuration>,
    pub delay: RefOr<TransitionDelay>,
    pub timing_function: RefOr<TransitionTimingFunction>,
}

impl<'a> TryFromJson<'a> for TransitionTokenValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &str,
        value: &'a serde_json::Value,
    ) -> Option<Self> {
        let obj = require_object(ctx, path, value, "transition token")?;
        let duration = obj.required_field::<RefOr<TransitionDuration>>(ctx, path, "duration");
        let delay = obj.required_field::<RefOr<TransitionDelay>>(ctx, path, "delay");
        let timing_function =
            obj.required_field::<RefOr<TransitionTimingFunction>>(ctx, path, "timingFunction");

        match (duration, delay, timing_function) {
            (Some(duration), Some(delay), Some(timing_function)) => Some(TransitionTokenValue {
                duration,
                delay,
                timing_function,
            }),
            _ => {
                ctx.push_to_errors(DiagnosticCode::InvalidTokenValue, "Expected 'duration', 'delay', and 'timingFunction' fields for transition token", path.into());
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        FileFormat, ParserContext,
        ir::{JsonPointer, JsonRef, RefOr, TokenAlias},
        token::token_types::composite::AliasOrLiteral,
    };
    use serde_json::json;

    fn parser_context() -> ParserContext {
        ParserContext::new("tests.json".into(), FileFormat::Json, String::new())
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    enum FieldMode {
        Literal,
        Alias,
        Ref,
    }

    fn duration_json(mode: FieldMode) -> serde_json::Value {
        match mode {
            FieldMode::Literal => json!({ "value": 200, "unit": "ms" }),
            FieldMode::Alias => json!("{motion.duration.fast}"),
            FieldMode::Ref => json!({ "$ref": "#/transition/duration" }),
        }
    }

    fn delay_json(mode: FieldMode) -> serde_json::Value {
        match mode {
            FieldMode::Literal => json!({ "value": 50, "unit": "ms" }),
            FieldMode::Alias => json!("{motion.delay.short}"),
            FieldMode::Ref => json!({ "$ref": "#/transition/delay" }),
        }
    }

    fn timing_json(mode: FieldMode) -> serde_json::Value {
        match mode {
            FieldMode::Literal => json!([0.25, 0.1, 0.25, 1.0]),
            FieldMode::Alias => json!("{motion.easing.standard}"),
            FieldMode::Ref => json!({ "$ref": "#/transition/timingFunction" }),
        }
    }

    fn classify_duration(value: &RefOr<TransitionDuration>) -> FieldMode {
        match value {
            RefOr::Ref(_) => FieldMode::Ref,
            RefOr::Literal(TransitionDuration(AliasOrLiteral::Alias(_))) => FieldMode::Alias,
            RefOr::Literal(TransitionDuration(AliasOrLiteral::Literal(_))) => FieldMode::Literal,
        }
    }

    fn classify_delay(value: &RefOr<TransitionDelay>) -> FieldMode {
        match value {
            RefOr::Ref(_) => FieldMode::Ref,
            RefOr::Literal(TransitionDelay(AliasOrLiteral::Alias(_))) => FieldMode::Alias,
            RefOr::Literal(TransitionDelay(AliasOrLiteral::Literal(_))) => FieldMode::Literal,
        }
    }

    fn classify_timing(value: &RefOr<TransitionTimingFunction>) -> FieldMode {
        match value {
            RefOr::Ref(_) => FieldMode::Ref,
            RefOr::Literal(TransitionTimingFunction(AliasOrLiteral::Alias(_))) => FieldMode::Alias,
            RefOr::Literal(TransitionTimingFunction(AliasOrLiteral::Literal(_))) => {
                FieldMode::Literal
            }
        }
    }

    #[test]
    fn parses_transition_duration_as_literal_and_alias() {
        let mut ctx = parser_context();

        let literal = TransitionDuration::try_from_json(
            &mut ctx,
            "/token/duration",
            &duration_json(FieldMode::Literal),
        );
        let alias = TransitionDuration::try_from_json(
            &mut ctx,
            "/token/duration",
            &duration_json(FieldMode::Alias),
        );

        assert!(matches!(
            literal,
            Some(TransitionDuration(AliasOrLiteral::Literal(_)))
        ));
        assert_eq!(
            alias,
            Some(TransitionDuration(AliasOrLiteral::Alias(
                TokenAlias::from_dtcg_alias("{motion.duration.fast}").unwrap()
            )))
        );
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn parses_transition_delay_as_literal_and_alias() {
        let mut ctx = parser_context();

        let literal = TransitionDelay::try_from_json(
            &mut ctx,
            "/token/delay",
            &delay_json(FieldMode::Literal),
        );
        let alias =
            TransitionDelay::try_from_json(&mut ctx, "/token/delay", &delay_json(FieldMode::Alias));

        assert!(matches!(
            literal,
            Some(TransitionDelay(AliasOrLiteral::Literal(_)))
        ));
        assert_eq!(
            alias,
            Some(TransitionDelay(AliasOrLiteral::Alias(
                TokenAlias::from_dtcg_alias("{motion.delay.short}").unwrap()
            )))
        );
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn parses_transition_timing_function_as_literal_and_alias() {
        let mut ctx = parser_context();

        let literal = TransitionTimingFunction::try_from_json(
            &mut ctx,
            "/token/timingFunction",
            &timing_json(FieldMode::Literal),
        );
        let alias = TransitionTimingFunction::try_from_json(
            &mut ctx,
            "/token/timingFunction",
            &timing_json(FieldMode::Alias),
        );

        assert!(matches!(
            literal,
            Some(TransitionTimingFunction(AliasOrLiteral::Literal(_)))
        ));
        assert_eq!(
            alias,
            Some(TransitionTimingFunction(AliasOrLiteral::Alias(
                TokenAlias::from_dtcg_alias("{motion.easing.standard}").unwrap()
            )))
        );
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn rejects_invalid_literal_transition_subfields() {
        let mut ctx = parser_context();

        let duration = TransitionDuration::try_from_json(&mut ctx, "/token/duration", &json!(42));
        let delay = TransitionDelay::try_from_json(&mut ctx, "/token/delay", &json!("later"));
        let timing = TransitionTimingFunction::try_from_json(
            &mut ctx,
            "/token/timingFunction",
            &json!(true),
        );

        assert_eq!(duration, None);
        assert_eq!(delay, None);
        assert_eq!(timing, None);
        assert_eq!(ctx.errors.len(), 3);
        assert_eq!(ctx.errors[0].path, "/token/duration");
        assert_eq!(ctx.errors[1].path, "/token/delay");
        assert_eq!(ctx.errors[2].path, "/token/timingFunction");
    }

    #[test]
    fn parses_transition_token_with_literal_fields() {
        let mut ctx = parser_context();
        let value = json!({
            "duration": duration_json(FieldMode::Literal),
            "delay": delay_json(FieldMode::Literal),
            "timingFunction": timing_json(FieldMode::Literal)
        });

        let parsed = TransitionTokenValue::try_from_json(&mut ctx, "/token", &value);

        match parsed {
            Some(TransitionTokenValue {
                duration,
                delay,
                timing_function,
            }) => {
                assert!(matches!(
                    duration,
                    RefOr::Literal(TransitionDuration(AliasOrLiteral::Literal(_)))
                ));
                assert!(matches!(
                    delay,
                    RefOr::Literal(TransitionDelay(AliasOrLiteral::Literal(_)))
                ));
                assert!(matches!(
                    timing_function,
                    RefOr::Literal(TransitionTimingFunction(AliasOrLiteral::Literal(_)))
                ));
            }
            None => panic!("expected transition token to parse successfully"),
        }
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn parses_transition_token_with_field_references() {
        let mut ctx = parser_context();
        let value = json!({
            "duration": duration_json(FieldMode::Ref),
            "delay": delay_json(FieldMode::Ref),
            "timingFunction": timing_json(FieldMode::Ref)
        });

        let parsed = TransitionTokenValue::try_from_json(&mut ctx, "/token", &value).unwrap();

        assert_eq!(
            parsed.duration,
            RefOr::Ref(JsonRef::new_local_pointer(
                "#/transition/duration".into(),
                JsonPointer::from("#/transition/duration"),
            ))
        );
        assert_eq!(
            parsed.delay,
            RefOr::Ref(JsonRef::new_local_pointer(
                "#/transition/delay".into(),
                JsonPointer::from("#/transition/delay"),
            ))
        );
        assert_eq!(
            parsed.timing_function,
            RefOr::Ref(JsonRef::new_local_pointer(
                "#/transition/timingFunction".into(),
                JsonPointer::from("#/transition/timingFunction"),
            ))
        );
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn parses_all_transition_field_mode_combinations() {
        let modes = [FieldMode::Literal, FieldMode::Alias, FieldMode::Ref];

        for duration_mode in modes {
            for delay_mode in modes {
                for timing_mode in modes {
                    let mut ctx = parser_context();
                    let value = json!({
                        "duration": duration_json(duration_mode),
                        "delay": delay_json(delay_mode),
                        "timingFunction": timing_json(timing_mode),
                    });

                    let parsed = TransitionTokenValue::try_from_json(&mut ctx, "/token", &value)
                        .unwrap_or_else(|| {
                            panic!(
                                "expected transition token to parse for combination {:?}/{:?}/{:?}; errors: {:?}",
                                duration_mode, delay_mode, timing_mode, ctx.errors
                            )
                        });

                    assert_eq!(classify_duration(&parsed.duration), duration_mode);
                    assert_eq!(classify_delay(&parsed.delay), delay_mode);
                    assert_eq!(classify_timing(&parsed.timing_function), timing_mode);
                    assert!(
                        ctx.errors.is_empty(),
                        "unexpected errors for combination {:?}/{:?}/{:?}: {:?}",
                        duration_mode,
                        delay_mode,
                        timing_mode,
                        ctx.errors
                    );
                }
            }
        }
    }

    #[test]
    fn reports_missing_required_transition_fields() {
        let mut ctx = parser_context();

        let parsed = TransitionTokenValue::try_from_json(&mut ctx, "/token", &json!({}));

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 4);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[0].path, "/token/duration");
        assert_eq!(ctx.errors[1].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[1].path, "/token/delay");
        assert_eq!(ctx.errors[2].code, DiagnosticCode::MissingRequiredProperty);
        assert_eq!(ctx.errors[2].path, "/token/timingFunction");
        assert_eq!(ctx.errors[3].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(ctx.errors[3].path, "/token");
    }

    #[test]
    fn reports_invalid_transition_field_and_missing_peers() {
        let mut ctx = parser_context();

        let parsed = TransitionTokenValue::try_from_json(
            &mut ctx,
            "/token",
            &json!({
                "duration": 42
            }),
        );

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 4);
        assert_eq!(ctx.errors[0].path, "/token/duration");
        assert_eq!(ctx.errors[1].path, "/token/delay");
        assert_eq!(ctx.errors[2].path, "/token/timingFunction");
        assert_eq!(ctx.errors[3].code, DiagnosticCode::InvalidTokenValue);
        assert_eq!(ctx.errors[3].path, "/token");
    }

    #[test]
    fn reports_invalid_top_level_shape_for_non_object_transition_token() {
        let mut ctx = parser_context();

        let parsed = TransitionTokenValue::try_from_json(&mut ctx, "/token", &json!("fast"));

        assert_eq!(parsed, None);
        assert_eq!(ctx.errors.len(), 1);
        assert_eq!(ctx.errors[0].code, DiagnosticCode::InvalidPropertyType);
        assert_eq!(ctx.errors[0].path, "/token");
    }
}
