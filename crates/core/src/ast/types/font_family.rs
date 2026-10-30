//! The `font_family` module provides the AST representation of fontFamily types in DTCG.
//!
//! A font family is a font name, or a non-empty list of font names from most to least preferred. A lone name is
//! read as a list of one. Each name in a list can be a `$ref` that leads to a string. A name is never a
//! `{alias}`: an alias is a whole token.

use crate::{
    ast::{
        node::{Node, PropId},
        types::FromProp,
        value::{ValueOrRef, alias_help, node, string_slot},
    },
    diagnostics::{
        code::DiagnosticCode,
        diagnostic::{Diagnostic, DiagnosticSink},
    },
    ir::{pipeline::Analyzed, refs::is_alias_like},
};

#[derive(Debug, Clone, PartialEq)]
pub struct FontFamily {
    /// In order of preference. Never empty.
    pub names: Vec<Node<ValueOrRef<String>>>,
}

impl FromProp for FontFamily {
    fn from_prop(prop: PropId, a: &Analyzed, diags: &mut impl DiagnosticSink) -> Option<Self> {
        if let Some(items) = a.items(prop) {
            if items.is_empty() {
                diags.emit(
                    Diagnostic::new(DiagnosticCode::InvalidTokenValue)
                        .with_message("a font family list must have at least one name")
                        .primary(a.span(prop), "add a font name, or use a single string"),
                );
                return None;
            }
            // Every item is read, so one run reports every problem in the list.
            let names: Vec<_> = items
                .iter()
                .map(|&item| string_slot(a, item, "a font name", diags))
                .collect();
            return Some(FontFamily {
                names: names.into_iter().collect::<Option<_>>()?,
            });
        }

        match a.string(prop) {
            Some(name) if !is_alias_like(name) => Some(FontFamily {
                names: vec![node(a, prop, ValueOrRef::Value(name.to_string()))],
            }),
            _ => {
                diags.emit(
                    Diagnostic::new(DiagnosticCode::InvalidTokenValue)
                        .with_message("a font family must be a font name or a list of font names")
                        .primary(
                            a.span(prop),
                            "expected \"Inter\" or [\"Inter\", \"sans-serif\"]",
                        )
                        .help(alias_help(a, prop)),
                );
                None
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::types::testing::{codes, ok, parse};

    fn names(f: &FontFamily) -> Vec<String> {
        f.names
            .iter()
            .map(|n| match &n.value {
                ValueOrRef::Value(s) => s.clone(),
                other => panic!("not a literal: {other:?}"),
            })
            .collect()
    }

    #[test]
    fn a_lone_name_is_a_list_of_one() {
        let f = ok::<FontFamily>("fontFamily", r#""Inter""#, "");
        assert_eq!(names(&f), ["Inter"]);
    }

    #[test]
    fn a_list_keeps_its_order() {
        let f = ok::<FontFamily>(
            "fontFamily",
            r#"["Inter", "Helvetica Neue", "sans-serif"]"#,
            "",
        );
        assert_eq!(names(&f), ["Inter", "Helvetica Neue", "sans-serif"]);
    }

    #[test]
    fn an_empty_list_is_rejected() {
        let (f, diags, _) = parse::<FontFamily>("fontFamily", "[]", "");
        assert!(f.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidTokenValue]);
    }

    #[test]
    fn a_non_string_is_rejected_with_an_example() {
        for value in ["12", "true", "null", r#"{"a": 1}"#] {
            let (f, diags, _) = parse::<FontFamily>("fontFamily", value, "");
            assert!(f.is_none(), "{value}");
            assert_eq!(
                codes(&diags),
                [DiagnosticCode::InvalidTokenValue],
                "{value}"
            );
        }
    }

    #[test]
    fn every_bad_item_is_reported_in_one_run() {
        let (f, diags, _) = parse::<FontFamily>("fontFamily", r#"[1, "ok", true]"#, "");
        assert!(f.is_none());
        assert_eq!(diags.diagnostics().len(), 2);
    }

    #[test]
    fn an_alias_is_not_a_font_name() {
        let (f, diags, _) = parse::<FontFamily>(
            "fontFamily",
            r#"["{other}"]"#,
            ", \"other\": {\"$type\": \"fontFamily\", \"$value\": \"Inter\"}",
        );
        assert!(f.is_none());
        let help = diags.diagnostics()[0].help.as_ref().unwrap();
        assert!(help.contains("$ref"), "{help}");
    }

    #[test]
    fn an_item_can_be_a_reference_to_a_string() {
        let f = ok::<FontFamily>(
            "fontFamily",
            r##"[{"$ref": "#/brand/$value"}, "sans-serif"]"##,
            r#", "brand": {"$type": "fontFamily", "$value": "Inter"}"#,
        );
        assert!(matches!(f.names[0].value, ValueOrRef::Reference(_)));
        assert!(matches!(f.names[1].value, ValueOrRef::Value(_)));
    }

    #[test]
    fn a_reference_to_a_non_string_is_reported_at_both_ends() {
        let (f, diags, _) = parse::<FontFamily>(
            "fontFamily",
            r##"[{"$ref": "#/n/$value"}]"##,
            r#", "n": {"$type": "number", "$value": 4}"#,
        );
        assert!(f.is_none());
        assert_eq!(diags.diagnostics()[0].labels.len(), 2);
    }
}
