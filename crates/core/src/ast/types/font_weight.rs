//! The `font_weight` module provides the AST representation of fontWeight types in DTCG.
//!
//! A font weight is a number from 1 to 1000 (the OpenType `wght` axis), or one of the named weights. A name is
//! turned into its number when parsed, so the rest of the compiler only sees numbers. `"normal"` is 400 and
//! `"bold"` is 700, for example.

use crate::{
    ast::{node::PropId, types::FromProp, value::alias_help},
    diagnostics::{
        code::DiagnosticCode,
        diagnostic::{Diagnostic, DiagnosticSink},
    },
    ir::pipeline::Analyzed,
};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FontWeight(pub f64);

/// Every named weight and its number.
const NAMED: [(&str, f64); 18] = [
    ("thin", 100.0),
    ("hairline", 100.0),
    ("extra-light", 200.0),
    ("ultra-light", 200.0),
    ("light", 300.0),
    ("normal", 400.0),
    ("regular", 400.0),
    ("book", 400.0),
    ("medium", 500.0),
    ("semi-bold", 600.0),
    ("demi-bold", 600.0),
    ("bold", 700.0),
    ("extra-bold", 800.0),
    ("ultra-bold", 800.0),
    ("black", 900.0),
    ("heavy", 900.0),
    ("extra-black", 950.0),
    ("ultra-black", 950.0),
];

impl FromProp for FontWeight {
    fn from_prop(prop: PropId, a: &Analyzed, diags: &mut impl DiagnosticSink) -> Option<Self> {
        if let Some(n) = a.number(prop) {
            if (1.0..=1000.0).contains(&n) {
                return Some(FontWeight(n));
            }
            diags.emit(
                Diagnostic::new(DiagnosticCode::InvalidTokenValue)
                    .with_message(format!("a font weight must be between 1 and 1000, not {n}"))
                    .primary(a.span(prop), "out of range")
                    .help("use 400 for normal text and 700 for bold, or a name such as \"bold\""),
            );
            return None;
        }

        if let Some(s) = a.string(prop) {
            if let Some((_, n)) = NAMED.iter().find(|(name, _)| *name == s) {
                return Some(FontWeight(*n));
            }
        }

        let names = NAMED.map(|(name, _)| format!("\"{name}\"")).join(", ");
        diags.emit(
            Diagnostic::new(DiagnosticCode::InvalidTokenValue)
                .with_message("a font weight must be a number from 1 to 1000 or a weight name")
                .primary(a.span(prop), "expected 400, 700 or a weight name")
                .help(format!("the names are {names}. {}", alias_help(a, prop))),
        );
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::types::testing::{codes, ok, parse};

    #[test]
    fn numbers_between_1_and_1000_are_kept() {
        for n in ["1", "400", "700", "1000", "350.5"] {
            let w = ok::<FontWeight>("fontWeight", n, "");
            assert_eq!(w.0, n.parse::<f64>().unwrap());
        }
    }

    #[test]
    fn numbers_outside_the_range_are_rejected() {
        for n in ["0", "0.5", "1001", "-100"] {
            let (w, diags, _) = parse::<FontWeight>("fontWeight", n, "");
            assert!(w.is_none(), "{n}");
            assert_eq!(codes(&diags), [DiagnosticCode::InvalidTokenValue], "{n}");
        }
    }

    #[test]
    fn every_name_maps_to_its_number() {
        for (name, n) in NAMED {
            let w = ok::<FontWeight>("fontWeight", &format!("\"{name}\""), "");
            assert_eq!(w.0, n, "{name}");
        }
    }

    #[test]
    fn the_common_names_have_their_conventional_numbers() {
        let weight = |name: &str| ok::<FontWeight>("fontWeight", &format!("\"{name}\""), "").0;
        assert_eq!(weight("normal"), 400.0);
        assert_eq!(weight("bold"), 700.0);
        assert_eq!(weight("semi-bold"), 600.0);
    }

    #[test]
    fn an_unknown_name_lists_the_known_ones() {
        let (w, diags, _) = parse::<FontWeight>("fontWeight", r#""chunky""#, "");
        assert!(w.is_none());
        let help = diags.diagnostics()[0].help.as_ref().unwrap();
        assert!(help.contains("\"semi-bold\""), "{help}");
    }

    #[test]
    fn names_are_case_sensitive() {
        let (w, _, _) = parse::<FontWeight>("fontWeight", r#""Bold""#, "");
        assert!(w.is_none());
    }

    #[test]
    fn other_shapes_are_rejected() {
        for v in ["true", "null", "[400]", r#"{"a": 1}"#] {
            let (w, diags, _) = parse::<FontWeight>("fontWeight", v, "");
            assert!(w.is_none(), "{v}");
            assert_eq!(codes(&diags), [DiagnosticCode::InvalidTokenValue], "{v}");
        }
    }

    #[test]
    fn an_alias_string_gets_the_alias_advice() {
        let (_, diags, _) = parse::<FontWeight>("fontWeight", r#""{w}""#, "");
        let help = diags
            .diagnostics()
            .iter()
            .find_map(|d| d.help.as_ref())
            .unwrap();
        assert!(help.contains("$ref"), "{help}");
    }
}
