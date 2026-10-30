//! Inputs: which context to use for each modifier.

use indexmap::IndexMap;

use crate::{
    diagnostics::{
        code::DiagnosticCode,
        diagnostic::{Diagnostic, DiagnosticSink, DiagnosticStore},
    },
    resolver::document::ResolverDocument,
};

/// A map of modifier name to the context to apply. Names and contexts are matched exactly first, then
/// case-insensitively when that is unambiguous.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Input {
    values: IndexMap<String, String>,
}

/// An input after validation: every modifier of the document mapped to a context that exists, in canonical case.
pub type Selection = IndexMap<String, String>;

impl Input {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with(mut self, modifier: impl Into<String>, context: impl Into<String>) -> Self {
        self.values.insert(modifier.into(), context.into());
        self
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &str)> {
        self.values.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }

    /// Reads a JSON object of strings, the form hosts receive from a CLI flag or a JS call. Non-string values are
    /// reported, since contexts are always strings.
    pub fn from_json(value: &serde_json::Value, diags: &mut DiagnosticStore) -> Option<Self> {
        let Some(object) = value.as_object() else {
            diags.emit(
                Diagnostic::new(DiagnosticCode::InvalidResolverInput)
                    .with_message("an input must be an object of modifier names to context names"),
            );
            return None;
        };
        let mut input = Self::new();
        let mut ok = true;
        for (k, v) in object {
            match v.as_str() {
                Some(s) => {
                    input.values.insert(k.clone(), s.to_string());
                }
                None => {
                    ok = false;
                    diags.emit(
                        Diagnostic::new(DiagnosticCode::InvalidResolverInput)
                            .with_message(format!("the value for modifier `{k}` must be a string")),
                    );
                }
            }
        }
        ok.then_some(input)
    }

    /// Checks the input against the document and fills in defaults.
    pub fn select(&self, doc: &ResolverDocument, diags: &mut DiagnosticStore) -> Option<Selection> {
        let mut chosen: IndexMap<String, String> = IndexMap::new();
        let mut ok = true;

        for (key, context) in &self.values {
            let Some(name) = match_name(doc.modifiers.keys().map(String::as_str), key) else {
                ok = false;
                diags.emit(
                    Diagnostic::new(DiagnosticCode::InvalidResolverInput)
                        .with_message(format!("`{key}` is not a modifier in this resolver"))
                        .help(format!(
                            "modifiers: {}",
                            doc.modifiers.keys().cloned().collect::<Vec<_>>().join(", ")
                        )),
                );
                continue;
            };
            let modifier = &doc.modifiers[&name];
            match match_name(modifier.contexts.keys().map(String::as_str), context) {
                Some(ctx) => {
                    chosen.insert(name, ctx);
                }
                None => {
                    ok = false;
                    diags.emit(
                        Diagnostic::new(DiagnosticCode::InvalidResolverInput)
                            .with_message(format!(
                                "`{context}` is not a context of modifier `{name}`"
                            ))
                            .help(format!(
                                "contexts: {}",
                                modifier
                                    .contexts
                                    .keys()
                                    .cloned()
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            )),
                    );
                }
            }
        }

        let mut out = Selection::new();
        for (name, modifier) in &doc.modifiers {
            if let Some(ctx) = chosen.shift_remove(name) {
                out.insert(name.clone(), ctx);
            } else if let Some(default) = &modifier.default {
                out.insert(name.clone(), default.clone());
            } else {
                ok = false;
                diags.emit(
                    Diagnostic::new(DiagnosticCode::InvalidResolverInput)
                        .with_message(format!(
                            "modifier `{name}` has no default, so an input is required"
                        ))
                        .help(format!(
                            "contexts: {}",
                            modifier
                                .contexts
                                .keys()
                                .cloned()
                                .collect::<Vec<_>>()
                                .join(", ")
                        )),
                );
            }
        }
        ok.then_some(out)
    }
}

fn match_name<'a>(
    candidates: impl Iterator<Item = &'a str> + Clone,
    wanted: &str,
) -> Option<String> {
    if let Some(exact) = candidates.clone().find(|c| *c == wanted) {
        return Some(exact.to_string());
    }
    let mut folded = candidates.filter(|c| c.eq_ignore_ascii_case(wanted));
    let first = folded.next()?;
    folded.next().is_none().then(|| first.to_string())
}

/// Every combination of contexts, for producing all permutations of a resolver.
pub fn all_inputs(doc: &ResolverDocument) -> Vec<Input> {
    let mut acc = vec![Input::new()];
    for (name, modifier) in &doc.modifiers {
        acc = acc
            .into_iter()
            .flat_map(|base| {
                modifier
                    .contexts
                    .keys()
                    .map(move |ctx| base.clone().with(name.clone(), ctx.clone()))
            })
            .collect();
    }
    acc
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{resolver::document, source::SourceIdGenerator};

    fn doc() -> ResolverDocument {
        let (tree, _) = json_spanned::parse_json(
            r##"{"version":"2025.10","modifiers":{
                "theme":{"default":"light","contexts":{"light":[],"dark":[]}},
                "size":{"contexts":{"sm":[],"lg":[]}}},
            "resolutionOrder":[{"$ref":"#/modifiers/theme"},{"$ref":"#/modifiers/size"}]}"##,
        );
        let mut d = DiagnosticStore::new();
        document::parse(
            SourceIdGenerator::default().generate(),
            "r.json",
            &tree.unwrap(),
            &mut d,
        )
        .unwrap()
    }

    fn select(input: Input) -> (Option<Selection>, DiagnosticStore) {
        let mut d = DiagnosticStore::new();
        (input.select(&doc(), &mut d), d)
    }

    #[test]
    fn applies_defaults_and_explicit_values() {
        let (s, d) = select(Input::new().with("size", "lg"));
        assert!(!d.has_errors());
        let s = s.unwrap();
        assert_eq!(s["theme"], "light");
        assert_eq!(s["size"], "lg");
        let (s, _) = select(Input::new().with("size", "sm").with("theme", "dark"));
        assert_eq!(s.unwrap()["theme"], "dark");
    }

    #[test]
    fn missing_input_without_default_is_an_error() {
        let (s, d) = select(Input::new());
        assert!(s.is_none());
        assert_eq!(
            d.diagnostics()[0].code,
            DiagnosticCode::InvalidResolverInput
        );
    }

    #[test]
    fn unknown_modifier_or_context_is_an_error() {
        assert!(
            select(Input::new().with("size", "lg").with("nope", "x"))
                .0
                .is_none()
        );
        assert!(select(Input::new().with("size", "xl")).0.is_none());
    }

    #[test]
    fn matching_is_case_insensitive_when_unambiguous() {
        let s = select(Input::new().with("SIZE", "LG")).0.unwrap();
        assert_eq!(s["size"], "lg");
    }

    #[test]
    fn from_json_requires_string_values() {
        let mut d = DiagnosticStore::new();
        let i = Input::from_json(&serde_json::json!({"size": "lg"}), &mut d).unwrap();
        assert_eq!(i.iter().collect::<Vec<_>>(), [("size", "lg")]);
        assert!(Input::from_json(&serde_json::json!({"size": 1}), &mut d).is_none());
        assert!(Input::from_json(&serde_json::json!([1]), &mut d).is_none());
        assert_eq!(d.error_count(), 2);
    }

    #[test]
    fn all_inputs_is_the_cartesian_product() {
        let all = all_inputs(&doc());
        assert_eq!(all.len(), 4);
        assert!(
            all.iter()
                .all(|i| i.select(&doc(), &mut DiagnosticStore::new()).is_some())
        );
    }
}
