//! The `transition` module provides the AST representation of transition types in DTCG.
//!
//! A transition is an object with `duration`, `delay` and `timingFunction`, all required. Each is a whole value of
//! its own type (duration, duration and cubicBezier), so each can be a literal, a `$ref` that leads to one, or a
//! `{token}` alias to a token of that type.

use crate::{
    ast::{
        node::{Node, PropId},
        types::{FromProp, TokenType, cubic_bezier::CubicBezier, duration::Duration},
        value::{ValueRefOrAlias, composite_field, object_members, reject_unknown},
    },
    diagnostics::diagnostic::DiagnosticSink,
    ir::pipeline::Analyzed,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Transition {
    pub duration: Node<ValueRefOrAlias<Duration>>,
    pub delay: Node<ValueRefOrAlias<Duration>>,
    pub timing_function: Node<ValueRefOrAlias<CubicBezier>>,
}

impl Transition {
    const KEYS: [&'static str; 3] = ["duration", "delay", "timingFunction"];
}

impl FromProp for Transition {
    fn from_prop(prop: PropId, a: &Analyzed, diags: &mut impl DiagnosticSink) -> Option<Self> {
        let members = object_members(
            a,
            prop,
            "transition",
            "{\"duration\": ..., \"delay\": ..., \"timingFunction\": ...}",
            diags,
        )?;
        reject_unknown(a, members, "transition", &Self::KEYS, diags);

        let field = "transition";
        let duration =
            composite_field::<Duration>(a, prop, field, "duration", TokenType::Duration, diags);
        let delay =
            composite_field::<Duration>(a, prop, field, "delay", TokenType::Duration, diags);
        let timing_function = composite_field::<CubicBezier>(
            a,
            prop,
            field,
            "timingFunction",
            TokenType::CubicBezier,
            diags,
        );

        Some(Transition {
            duration: duration?,
            delay: delay?,
            timing_function: timing_function?,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ast::types::testing::{codes, ok, parse},
        diagnostics::code::DiagnosticCode,
    };

    const MS: &str = r#"{"value": 200, "unit": "ms"}"#;
    const EASE: &str = "[0.25, 0.1, 0.25, 1]";

    fn json(extra: &str) -> String {
        format!(r#"{{"duration": {MS}, "delay": {MS}, "timingFunction": {EASE}{extra}}}"#)
    }

    #[test]
    fn parses_a_transition() {
        let t = ok::<Transition>("transition", &json(""), "");
        assert!(matches!(t.duration.value, ValueRefOrAlias::Value(_)));
        assert!(matches!(t.delay.value, ValueRefOrAlias::Value(_)));
        assert!(matches!(t.timing_function.value, ValueRefOrAlias::Value(_)));
    }

    #[test]
    fn all_three_members_are_required_and_reported_together() {
        let (t, diags, _) = parse::<Transition>("transition", "{}", "");
        assert!(t.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidPropertyShape; 3]);
    }

    #[test]
    fn a_non_object_is_rejected() {
        for v in ["4", r#""ease""#, "[1]", "null"] {
            let (t, diags, _) = parse::<Transition>("transition", v, "");
            assert!(t.is_none(), "{v}");
            assert_eq!(codes(&diags), [DiagnosticCode::InvalidPropertyShape], "{v}");
        }
    }

    #[test]
    fn an_unknown_member_is_reported_but_known_ones_are_still_read() {
        let (t, diags, _) = parse::<Transition>("transition", &json(r#", "easing": 1"#), "");
        assert!(t.is_some());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidPropertyShape]);
    }

    #[test]
    fn members_can_be_aliases_and_references() {
        let v = r##"{"duration": "{fast}", "delay": {"$ref": "#/fast/$value"}, "timingFunction": "{ease}"}"##;
        let t = ok::<Transition>(
            "transition",
            v,
            &format!(
                r#", "fast": {{"$type": "duration", "$value": {MS}}}, "ease": {{"$type": "cubicBezier", "$value": {EASE}}}"#
            ),
        );
        assert!(matches!(t.duration.value, ValueRefOrAlias::Alias(_)));
        assert!(matches!(t.delay.value, ValueRefOrAlias::Reference(_)));
        assert!(matches!(t.timing_function.value, ValueRefOrAlias::Alias(_)));
    }

    #[test]
    fn a_duration_member_cannot_alias_a_dimension() {
        let v = format!(r#"{{"duration": "{{d}}", "delay": {MS}, "timingFunction": {EASE}}}"#);
        let (t, diags, _) = parse::<Transition>(
            "transition",
            &v,
            r#", "d": {"$type": "dimension", "$value": {"value": 1, "unit": "px"}}"#,
        );
        assert!(t.is_none());
        assert_eq!(diags.diagnostics()[0].labels.len(), 2);
    }

    #[test]
    fn a_bad_member_is_reported_and_the_others_are_still_checked() {
        let v = format!(r#"{{"duration": 5, "delay": {MS}, "timingFunction": [2, 0, 0, 0]}}"#);
        let (t, diags, _) = parse::<Transition>("transition", &v, "");
        assert!(t.is_none());
        assert_eq!(diags.diagnostics().len(), 2);
    }
}
