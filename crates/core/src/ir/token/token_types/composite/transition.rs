//! The `transition` module defines the `TransitionTokenValue` struct, which represents a transition token in DTCG.

use crate::{
    errors::DiagnosticCode,
    ir::token::token_types::{cubic_bezier::CubicBezierTokenValue, duration::DurationTokenValue},
    ir::{
        DiagnosticEmission, InvalidReason, JsonObject, ParseState, RefAliasOrLiteral, TryFromJson,
    },
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TransitionDuration(pub RefAliasOrLiteral<DurationTokenValue>);

impl<'a> TryFromJson<'a> for TransitionDuration {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &JsonPointer,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match RefAliasOrLiteral::<DurationTokenValue>::try_from_json(ctx, path, value) {
            ParseState::Parsed(val) => ParseState::Parsed(Self(val)),
            ParseState::Failed(inv) => {
                if inv.ownership == DiagnosticEmission::Silent {
                    ctx.push_to_errors(DiagnosticCode::InvalidPropertyType, format!("Expected a DTCG reference, a JSON reference, or a duration token but got {:?}", value), path.into());
                    return ParseState::invalid_emitted(InvalidReason::InvalidValue);
                }
                ParseState::Failed(inv)
            }
            ParseState::NoMatch => {
                ctx.push_to_errors(DiagnosticCode::InvalidPropertyType, format!("Expected a DTCG reference, a JSON reference, or a duration token but got {:?}", value), path.into());
                ParseState::invalid_emitted(InvalidReason::InvalidValue)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TransitionDelay(pub RefAliasOrLiteral<DurationTokenValue>);

impl<'a> TryFromJson<'a> for TransitionDelay {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &JsonPointer,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match RefAliasOrLiteral::<DurationTokenValue>::try_from_json(ctx, path, value) {
            ParseState::Parsed(val) => ParseState::Parsed(Self(val)),
            ParseState::Failed(inv) => {
                if inv.ownership == DiagnosticEmission::Silent {
                    ctx.push_to_errors(DiagnosticCode::InvalidPropertyType, format!("Expected a DTCG reference, a JSON reference, or a duration token but got {:?}", value), path.into());
                    return ParseState::invalid_emitted(InvalidReason::InvalidValue);
                }
                ParseState::Failed(inv)
            }
            ParseState::NoMatch => {
                ctx.push_to_errors(DiagnosticCode::InvalidPropertyType, format!("Expected a DTCG reference, a JSON reference, or a duration token but got {:?}", value), path.into());
                ParseState::invalid_emitted(InvalidReason::InvalidValue)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TransitionTimingFunction(pub RefAliasOrLiteral<CubicBezierTokenValue>);

impl<'a> TryFromJson<'a> for TransitionTimingFunction {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &JsonPointer,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        match RefAliasOrLiteral::<CubicBezierTokenValue>::try_from_json(ctx, path, value) {
            ParseState::Parsed(val) => ParseState::Parsed(Self(val)),
            ParseState::Failed(inv) => {
                if inv.ownership == DiagnosticEmission::Silent {
                    ctx.push_to_errors(DiagnosticCode::InvalidPropertyType, format!("Expected a DTCG reference, a JSON reference, or a cubic-bezier token but got {:?}", value), path.into());
                    return ParseState::invalid_emitted(InvalidReason::InvalidValue);
                }
                ParseState::Failed(inv)
            }
            ParseState::NoMatch => {
                ctx.push_to_errors(DiagnosticCode::InvalidPropertyType, format!("Expected a DTCG reference, a JSON reference, or a cubic-bezier token but got {:?}", value), path.into());
                ParseState::invalid_emitted(InvalidReason::InvalidValue)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TransitionTokenValue {
    pub duration: TransitionDuration,
    pub delay: TransitionDelay,
    pub timing_function: TransitionTimingFunction,
}

impl<'a> TryFromJson<'a> for TransitionTokenValue {
    fn try_from_json(
        ctx: &mut crate::ParserContext,
        path: &JsonPointer,
        value: &'a serde_json::Value,
    ) -> ParseState<Self> {
        let obj = match value {
            serde_json::Value::Object(map) => JsonObject::new(map),
            _ => return ParseState::NoMatch,
        };

        let duration = obj.required_field::<TransitionDuration>(ctx, path, "duration");
        let delay = obj.required_field::<TransitionDelay>(ctx, path, "delay");
        let timing_function =
            obj.required_field::<TransitionTimingFunction>(ctx, path, "timingFunction");

        match (duration, delay, timing_function) {
            (Some(duration), Some(delay), Some(timing_function)) => ParseState::Parsed(Self {
                duration,
                delay,
                timing_function,
            }),
            _ => return ParseState::invalid_emitted(InvalidReason::Other),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::{
        ParserContext,
        errors::DiagnosticCode,
        ir::{ParseState, RefAliasOrLiteral, TryFromJson},
    };

    fn test_ctx() -> ParserContext {
        ParserContext::new("{}".to_string())
    }

    #[test]
    fn transition_duration_parses_literal_ref_and_alias() {
        let mut ctx = test_ctx();

        let literal = TransitionDuration::try_from_json(
            &mut ctx,
            "#/token/duration",
            &json!({ "value": 300, "unit": "ms" }),
        );
        let by_ref = TransitionDuration::try_from_json(
            &mut ctx,
            "#/token/duration",
            &json!({ "$ref": "#/motion/fast/duration" }),
        );
        let alias = TransitionDuration::try_from_json(
            &mut ctx,
            "#/token/duration",
            &json!("{motion.fast.duration}"),
        );

        assert!(matches!(
            literal,
            ParseState::Parsed(TransitionDuration(RefAliasOrLiteral::Literal(_)))
        ));
        assert!(matches!(
            by_ref,
            ParseState::Parsed(TransitionDuration(RefAliasOrLiteral::Ref(_)))
        ));
        assert!(matches!(
            alias,
            ParseState::Parsed(TransitionDuration(RefAliasOrLiteral::Alias(_)))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn transition_delay_parses_literal_ref_and_alias() {
        let mut ctx = test_ctx();

        let literal = TransitionDelay::try_from_json(
            &mut ctx,
            "#/token/delay",
            &json!({ "value": 120, "unit": "ms" }),
        );
        let by_ref = TransitionDelay::try_from_json(
            &mut ctx,
            "#/token/delay",
            &json!({ "$ref": "#/motion/fast/delay" }),
        );
        let alias = TransitionDelay::try_from_json(
            &mut ctx,
            "#/token/delay",
            &json!("{motion.fast.delay}"),
        );

        assert!(matches!(
            literal,
            ParseState::Parsed(TransitionDelay(RefAliasOrLiteral::Literal(_)))
        ));
        assert!(matches!(
            by_ref,
            ParseState::Parsed(TransitionDelay(RefAliasOrLiteral::Ref(_)))
        ));
        assert!(matches!(
            alias,
            ParseState::Parsed(TransitionDelay(RefAliasOrLiteral::Alias(_)))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn transition_timing_function_parses_literal_ref_and_alias() {
        let mut ctx = test_ctx();

        let literal = TransitionTimingFunction::try_from_json(
            &mut ctx,
            "#/token/timingFunction",
            &json!([0.25, 0.1, 0.25, 1.0]),
        );
        let by_ref = TransitionTimingFunction::try_from_json(
            &mut ctx,
            "#/token/timingFunction",
            &json!({ "$ref": "#/motion/easing/standard" }),
        );
        let alias = TransitionTimingFunction::try_from_json(
            &mut ctx,
            "#/token/timingFunction",
            &json!("{motion.easing.standard}"),
        );

        assert!(matches!(
            literal,
            ParseState::Parsed(TransitionTimingFunction(RefAliasOrLiteral::Literal(_)))
        ));
        assert!(matches!(
            by_ref,
            ParseState::Parsed(TransitionTimingFunction(RefAliasOrLiteral::Ref(_)))
        ));
        assert!(matches!(
            alias,
            ParseState::Parsed(TransitionTimingFunction(RefAliasOrLiteral::Alias(_)))
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn transition_token_value_parses_valid_object() {
        let mut ctx = test_ctx();
        let input = json!({
            "duration": { "value": 300, "unit": "ms" },
            "delay": { "value": 50, "unit": "ms" },
            "timingFunction": [0.25, 0.1, 0.25, 1.0]
        });

        let state = TransitionTokenValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::Parsed(_)));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn transition_token_value_parses_refs_and_aliases() {
        let mut ctx = test_ctx();
        let input = json!({
            "duration": { "$ref": "#/motion/fast/duration" },
            "delay": "{motion.fast.delay}",
            "timingFunction": { "$ref": "#/motion/easing/standard" }
        });

        let state = TransitionTokenValue::try_from_json(&mut ctx, "#/token", &input);

        let parsed = match state {
            ParseState::Parsed(v) => v,
            _ => panic!("expected parsed transition token value"),
        };

        assert!(matches!(parsed.duration.0, RefAliasOrLiteral::Ref(_)));
        assert!(matches!(parsed.delay.0, RefAliasOrLiteral::Alias(_)));
        assert!(matches!(
            parsed.timing_function.0,
            RefAliasOrLiteral::Ref(_)
        ));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn transition_token_value_returns_no_match_for_non_object() {
        let mut ctx = test_ctx();

        let state = TransitionTokenValue::try_from_json(&mut ctx, "#/token", &json!(42));

        assert!(matches!(state, ParseState::NoMatch));
        assert!(ctx.errors.is_empty());
    }

    #[test]
    fn transition_token_value_reports_missing_required_field() {
        let mut ctx = test_ctx();
        let input = json!({
            "duration": { "value": 300, "unit": "ms" },
            "delay": { "value": 50, "unit": "ms" }
        });

        let state = TransitionTokenValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::Failed(_)));
        assert!(ctx.errors.iter().any(|e| {
            e.code == DiagnosticCode::MissingRequiredProperty && e.path == "#/token/timingFunction"
        }));
    }

    #[test]
    fn transition_token_value_reports_invalid_field_value() {
        let mut ctx = test_ctx();
        let input = json!({
            "duration": true,
            "delay": { "value": 50, "unit": "ms" },
            "timingFunction": [0.25, 0.1, 0.25, 1.0]
        });

        let state = TransitionTokenValue::try_from_json(&mut ctx, "#/token", &input);

        assert!(matches!(state, ParseState::Failed(_)));
        assert!(ctx.errors.iter().any(|e| e.path == "#/token/duration"));
    }
}
