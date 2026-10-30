//! The `value` module holds what every typed value parser shares: how a slot inside a `$value` can be a reference.
//!
//! Inside a value, a property such as a dimension's `unit` is either a literal or a JSON-pointer reference object,
//! `{"$ref": "#/path"}`. A property that is itself a whole value, such as a shadow's `color`, can also be a
//! curly-brace alias to a token of that type (see [`composite_slot`]). Primitive properties cannot, and the token
//! dispatcher handles a whole token's alias before any typed parser runs.

use crate::{
    ast::{
        node::{Node, PropId},
        types::{FromProp, TokenType},
    },
    diagnostics::{
        code::DiagnosticCode,
        diagnostic::{Diagnostic, DiagnosticSink, DiagnosticStore},
    },
    ir::{
        lower::{PropKind, TokenBody},
        node::DtcgValueSyntax,
        pipeline::Analyzed,
        refs::is_alias_like,
        types::Resolved,
    },
};

/// A slot holds either a literal or a reference. The reference holds the *final literal* it leads to, after
/// following any chain of references, and the [`Node`] around it keeps the slot's own span, so both ends can be
/// shown in a diagnostic.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValueOrRef<T> {
    Value(T),
    Reference(PropId),
}

/// A member of a composite value. Unlike a primitive slot, the spec lets it name a whole token with `"{token}"`.
#[derive(Debug, Clone, PartialEq)]
pub enum ValueRefOrAlias<T> {
    Value(T),
    /// `{"$ref": ...}`. It holds the final literal the reference leads to.
    Reference(PropId),
    /// `"{token}"`. It holds the token the alias names, which has the type the slot needs.
    Alias(PropId),
}

/// What a slot turned out to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SlotKind {
    /// Not a reference. The caller reads it as a literal.
    Literal,
    /// A `$ref` object that resolved to this prop.
    Reference(PropId),
    /// A malformed or unresolved `$ref` object. It was reported already, so the caller should stay quiet.
    Broken,
}

/// Decides whether `slot` is a `$ref` object, without looking at any literal rules.
pub fn classify_slot(a: &Analyzed, slot: PropId, diags: &mut impl DiagnosticSink) -> SlotKind {
    let Some(members) = a.members(slot) else {
        return SlotKind::Literal;
    };
    if !members.contains_key("$ref") {
        return SlotKind::Literal;
    }

    if members.len() > 1 {
        diags.emit(
            Diagnostic::new(DiagnosticCode::InvalidPropertyShape)
                .with_message("a reference object must only contain `$ref`")
                .primary(a.span(slot), "remove the other members, or use a literal"),
        );
        return SlotKind::Broken;
    }

    match a.target(slot) {
        Some(target) => SlotKind::Reference(target),
        None => SlotKind::Broken,
    }
}

/// Wraps `value` with the identity and span of the slot it was read from.
pub fn node<T>(a: &Analyzed, slot: PropId, value: T) -> Node<T> {
    Node {
        value,
        id: slot,
        span: a.span(slot),
    }
}

/// Why following a reference did not end at a literal.
enum Follow {
    /// A token or group where a value was needed. Carries the prop that was reached.
    NotAValue(PropId),
    /// The chain ends in something already reported.
    Broken,
}

/// Follows a `$ref` from `start` to the literal it finally leads to.
///
/// A `$ref` must name a value, so landing on a token or group (rather than its `$value`) is an error and is never
/// quietly dereferenced. Aliases are different: `{token}` names the token's value by definition, so a chain may pass
/// through them. The graph is acyclic by the time this runs, so the loop ends.
fn follow(a: &Analyzed, start: PropId) -> Result<PropId, Follow> {
    if matches!(
        a.lowered.get(start).kind,
        PropKind::Token(_) | PropKind::Group(_)
    ) {
        return Err(Follow::NotAValue(start));
    }

    let mut p = start;
    while let Some(next) = a.target(p) {
        let via_alias = a.string(p).is_some();
        p = next;

        match &a.lowered.get(p).kind {
            PropKind::Group(_) => return Err(Follow::NotAValue(p)),
            PropKind::Token(_) if !via_alias => return Err(Follow::NotAValue(p)),
            PropKind::Token(data) => match data.body {
                TokenBody::Value(v) => p = v,
                // The token is itself a `$ref` holder, so the next turn of the loop follows it.
                TokenBody::Ref { .. } => {}
                TokenBody::Invalid => return Err(Follow::Broken),
            },
            PropKind::Value(_) => {}
        }
    }
    Ok(p)
}

/// Follows the reference in `slot` and checks that the literal at the end is acceptable to `accepts`.
/// Reports a mismatch with the slot as the main label and the literal as the secondary one.
pub(crate) fn checked_reference(
    a: &Analyzed,
    slot: PropId,
    target: PropId,
    expected: &str,
    accepts: impl FnOnce(PropId) -> bool,
    diags: &mut impl DiagnosticSink,
) -> Option<PropId> {
    match follow(a, target) {
        Ok(end) if accepts(end) => Some(end),
        Ok(end) => {
            diags.emit(
                Diagnostic::new(DiagnosticCode::InvalidTokenValue)
                    .with_message(format!(
                        "this reference leads to a value that is not {expected}"
                    ))
                    .primary(a.span(slot), format!("expected {expected}"))
                    .secondary(a.span(end), "the reference ends here"),
            );
            None
        }
        Err(Follow::NotAValue(at)) => {
            let path = a.lowered.path_of(at);
            let help = match &a.lowered.get(at).kind {
                PropKind::Token(_) => {
                    format!("point at the token's value instead: `{path}/$value`")
                }
                _ => "a group has no value, point at a value inside it".to_string(),
            };
            diags.emit(
                Diagnostic::new(DiagnosticCode::InvalidTokenValue)
                    .with_message(format!(
                        "a `$ref` here must point at {expected}, but `{path}` is a {}",
                        if matches!(a.lowered.get(at).kind, PropKind::Token(_)) {
                            "token"
                        } else {
                            "group"
                        }
                    ))
                    .primary(a.span(slot), format!("expected {expected}"))
                    .secondary(a.span(at), "this is not a value")
                    .help(help),
            );
            None
        }
        Err(Follow::Broken) => None,
    }
}

/// Reads a member of a composite value that is itself a value of type `ty`, such as a shadow's `color`. It can be
/// a literal, a `$ref` that leads to a valid `T`, or a `{token}` alias to a token of type `ty`.
pub fn composite_slot<T: FromProp>(
    a: &Analyzed,
    slot: PropId,
    ty: TokenType,
    diags: &mut impl DiagnosticSink,
) -> Option<Node<ValueRefOrAlias<T>>> {
    match classify_slot(a, slot, diags) {
        SlotKind::Reference(target) => {
            // The target's own problems are the target's to report, so only the verdict is used here.
            let end = checked_reference(
                a,
                slot,
                target,
                &format!("a valid `{}` value", ty.as_str()),
                |p| T::from_prop(p, a, &mut DiagnosticStore::new()).is_some(),
                diags,
            )?;
            Some(node(a, slot, ValueRefOrAlias::Reference(end)))
        }
        SlotKind::Broken => None,
        SlotKind::Literal => match a.string(slot) {
            Some(s) if is_alias_like(s) => {
                let target = alias_target(a, slot, ty, diags)?;
                Some(node(a, slot, ValueRefOrAlias::Alias(target)))
            }
            _ => {
                let value = T::from_prop(slot, a, diags)?;
                Some(node(a, slot, ValueRefOrAlias::Value(value)))
            }
        },
    }
}

/// The token a `"{token}"` alias in `slot` names, if it is a token of type `ty`. An alias that did not resolve, or
/// whose target has no usable type, was reported by an earlier pass.
pub(crate) fn alias_target(
    a: &Analyzed,
    slot: PropId,
    ty: TokenType,
    diags: &mut impl DiagnosticSink,
) -> Option<PropId> {
    let target = a.target(slot)?;
    let Some(Resolved::Known { name, .. }) = a.types.get(target) else {
        return None;
    };
    if name == ty.as_str() {
        return Some(target);
    }
    diags.emit(
        Diagnostic::new(DiagnosticCode::InvalidTokenValue)
            .with_message(format!(
                "this alias leads to a `{name}` token, but a `{}` is needed here",
                ty.as_str()
            ))
            .primary(a.span(slot), format!("expected a `{}` token", ty.as_str()))
            .secondary(a.span(target), format!("this token is a `{name}`")),
    );
    None
}

/// Reads a slot that is a boolean or a reference.
pub fn bool_slot(
    a: &Analyzed,
    slot: PropId,
    what: &str,
    diags: &mut impl DiagnosticSink,
) -> Option<Node<ValueOrRef<bool>>> {
    let as_bool = |p| match a.syntax(p) {
        DtcgValueSyntax::Bool(b) => Some(*b),
        _ => None,
    };
    match classify_slot(a, slot, diags) {
        SlotKind::Reference(target) => {
            let end = checked_reference(
                a,
                slot,
                target,
                "a boolean",
                |p| as_bool(p).is_some(),
                diags,
            )?;
            Some(node(a, slot, ValueOrRef::Reference(end)))
        }
        SlotKind::Broken => None,
        SlotKind::Literal => match as_bool(slot) {
            Some(b) => Some(node(a, slot, ValueOrRef::Value(b))),
            None => {
                diags.emit(
                    Diagnostic::new(DiagnosticCode::InvalidTokenValue)
                        .with_message(format!("{what} must be true or false"))
                        .primary(a.span(slot), "expected true or false")
                        .help(alias_help(a, slot)),
                );
                None
            }
        },
    }
}

/// Reads a slot that is a number or a reference.
pub fn number_slot(
    a: &Analyzed,
    slot: PropId,
    what: &str,
    diags: &mut impl DiagnosticSink,
) -> Option<Node<ValueOrRef<f64>>> {
    match classify_slot(a, slot, diags) {
        SlotKind::Reference(target) => {
            let end = checked_reference(
                a,
                slot,
                target,
                "a number",
                |p| a.number(p).is_some(),
                diags,
            )?;
            Some(node(a, slot, ValueOrRef::Reference(end)))
        }
        SlotKind::Broken => None,
        SlotKind::Literal => match a.number(slot) {
            Some(n) => Some(node(a, slot, ValueOrRef::Value(n))),
            None => {
                diags.emit(
                    Diagnostic::new(DiagnosticCode::InvalidTokenValue)
                        .with_message(format!("{what} must be a number"))
                        .primary(a.span(slot), "expected a number")
                        .help(alias_help(a, slot)),
                );
                None
            }
        },
    }
}

/// Reads a slot that is a string or a reference. A `{alias}` is not a string here, see [`alias_help`].
pub fn string_slot(
    a: &Analyzed,
    slot: PropId,
    what: &str,
    diags: &mut impl DiagnosticSink,
) -> Option<Node<ValueOrRef<String>>> {
    match classify_slot(a, slot, diags) {
        SlotKind::Reference(target) => {
            let end = checked_reference(
                a,
                slot,
                target,
                "a string",
                |p| a.string(p).is_some_and(|s| !is_alias_like(s)),
                diags,
            )?;
            Some(node(a, slot, ValueOrRef::Reference(end)))
        }
        SlotKind::Broken => None,
        SlotKind::Literal => match a.string(slot) {
            Some(s) if !is_alias_like(s) => Some(node(a, slot, ValueOrRef::Value(s.to_string()))),
            _ => {
                diags.emit(
                    Diagnostic::new(DiagnosticCode::InvalidTokenValue)
                        .with_message(format!("{what} must be a string"))
                        .primary(a.span(slot), "expected a string")
                        .help(alias_help(a, slot)),
                );
                None
            }
        },
    }
}

/// The members of `prop`, or a report that a composite `owner` has to be an object.
pub(crate) fn object_members<'a>(
    a: &'a Analyzed,
    prop: PropId,
    owner: &str,
    example: &str,
    diags: &mut impl DiagnosticSink,
) -> Option<&'a indexmap::IndexMap<String, PropId>> {
    let members = a.members(prop);
    if members.is_none() {
        diags.emit(
            Diagnostic::new(DiagnosticCode::InvalidPropertyShape)
                .with_message(format!("a {owner} must be an object"))
                .primary(a.span(prop), format!("expected {example}")),
        );
    }
    members
}

/// Reads the required member `key` of a composite `owner` (like "border") as a whole value of type `ty`.
pub(crate) fn composite_field<T: FromProp>(
    a: &Analyzed,
    prop: PropId,
    owner: &str,
    key: &str,
    ty: TokenType,
    diags: &mut impl DiagnosticSink,
) -> Option<Node<ValueRefOrAlias<T>>> {
    match a.member(prop, key) {
        Some(slot) => composite_slot(a, slot, ty, diags),
        None => {
            diags.emit(missing_member(a, prop, owner, key));
            None
        }
    }
}

pub(crate) fn missing_member(a: &Analyzed, prop: PropId, owner: &str, key: &str) -> Diagnostic {
    Diagnostic::new(DiagnosticCode::InvalidPropertyShape)
        .with_message(format!("a {owner} is missing `{key}`"))
        .primary(a.span(prop), format!("add `{key}` here"))
}

/// Reports every member of `members` that the schema does not allow. It does not stop the known ones being read.
pub(crate) fn reject_unknown(
    a: &Analyzed,
    members: &indexmap::IndexMap<String, PropId>,
    owner: &str,
    keys: &[&str],
    diags: &mut impl DiagnosticSink,
) {
    for (key, &member) in members {
        if !keys.contains(&key.as_str()) {
            diags.emit(
                Diagnostic::new(DiagnosticCode::InvalidPropertyShape)
                    .with_message(format!("unknown {owner} property `{key}`"))
                    .primary(
                        a.span(member),
                        format!("a {owner} only has {}", quoted(keys)),
                    ),
            );
        }
    }
}

/// Reads a slot that is one of a fixed set of strings, or a reference. `parse` maps the string to the literal.
pub fn choice_slot<T>(
    a: &Analyzed,
    slot: PropId,
    what: &str,
    allowed: &[&str],
    parse: impl FnOnce(&str) -> Option<T>,
    diags: &mut impl DiagnosticSink,
) -> Option<Node<ValueOrRef<T>>> {
    match classify_slot(a, slot, diags) {
        SlotKind::Reference(target) => {
            let list = quoted(allowed);
            let end = checked_reference(
                a,
                slot,
                target,
                &format!("one of {list}"),
                |p| a.string(p).is_some_and(|s| allowed.contains(&s)),
                diags,
            )?;
            Some(node(a, slot, ValueOrRef::Reference(end)))
        }
        SlotKind::Broken => None,
        SlotKind::Literal => match a.string(slot).and_then(parse) {
            Some(v) => Some(node(a, slot, ValueOrRef::Value(v))),
            None => {
                let list = quoted(allowed);
                diags.emit(
                    Diagnostic::new(DiagnosticCode::InvalidTokenValue)
                        .with_message(format!("{what} must be one of {list}"))
                        .primary(a.span(slot), format!("expected one of {list}"))
                        .help(alias_help(a, slot)),
                );
                None
            }
        },
    }
}

fn quoted(allowed: &[&str]) -> String {
    allowed
        .iter()
        .map(|s| format!("\"{s}\""))
        .collect::<Vec<_>>()
        .join(", ")
}

/// Authors coming from `{alias}` syntax often try it inside a value, so say what to do instead.
pub(crate) fn alias_help(a: &Analyzed, slot: PropId) -> String {
    match a.string(slot) {
        Some(s) if is_alias_like(s) => {
            "`{alias}` references only work as a token's whole `$value`. Inside a value, use {\"$ref\": \"#/path/to/token/$value\"}".into()
        }
        _ => "a property can also be a reference object, {\"$ref\": \"#/path\"}".into(),
    }
}
