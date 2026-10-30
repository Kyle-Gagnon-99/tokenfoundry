//! The `resolve` module flattens the token tree for targets that have no variables or aliases.
//!
//! The [`TokenArena`] keeps aliases and references as written, because a CSS backend wants `var(--space-sm)`. This
//! pass is the other choice: every alias and `$ref` is followed to its literal, and the result has no reference
//! variants, so "nothing is left unresolved" is checked by the compiler. It only reads, so it can run as often as
//! a backend needs and the faithful tree stays intact.
//!
//! Building already checked that each reference ends at a valid value of the token's type, so a failure here only
//! means the token (or something it points at) was invalid, and that token is left out.

use indexmap::IndexMap;
use serde::Serialize;

use crate::{
    ast::{
        token::{AnyToken, Deprecation, RefAliasOrValue, TokenArena, TokenId},
        types::{
            FromProp, TokenType, TokenValue,
            border::Border,
            color::{Color, ColorSpace, Component, read_component},
            cubic_bezier::CubicBezier,
            dimension::{Dimension, DimensionUnit},
            duration::{Duration, DurationUnit},
            font_family::FontFamily,
            font_weight::FontWeight,
            gradient::{Gradient, GradientStop},
            number::Number,
            parse_token_value,
            shadow::{Shadow, ShadowLayer},
            stroke_style::{LineCap, StrokeKeyword, StrokeStyle, dash_array_items},
            transition::Transition,
            typography::Typography,
        },
        value::{ValueOrRef, ValueRefOrAlias},
    },
    diagnostics::diagnostic::DiagnosticStore,
    ir::{node::DtcgValueSyntax, pipeline::Analyzed},
};

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResolvedDimension {
    pub value: f64,
    pub unit: DimensionUnit,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedColor {
    pub color_space: ColorSpace,
    /// `None` is the `"none"` keyword, a missing component, which is not the same as 0.
    pub components: [Option<f64>; 3],
    /// 1 when the author left it out.
    pub alpha: f64,
    pub hex: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResolvedNumber(pub f64);

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResolvedDuration {
    pub value: f64,
    pub unit: DurationUnit,
}

/// `[P1x, P1y, P2x, P2y]`.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResolvedCubicBezier(pub [f64; 4]);

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedShadowLayer {
    pub color: ResolvedColor,
    pub offset_x: ResolvedDimension,
    pub offset_y: ResolvedDimension,
    pub blur: ResolvedDimension,
    pub spread: ResolvedDimension,
    /// False when the author left it out.
    pub inset: bool,
}

/// A shadow is always a list. A shadow that refers to other shadows has them flattened in, in order.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResolvedShadow {
    pub layers: Vec<ResolvedShadowLayer>,
}

/// Font names from most to least preferred. A lone name is a list of one.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResolvedFontFamily(pub Vec<String>);

/// The weight as a number from 1 to 1000. A weight name has already become its number.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResolvedFontWeight(pub f64);

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedDashedStroke {
    pub dash_array: Vec<ResolvedDimension>,
    pub line_cap: LineCap,
}

/// A style name serializes as that string and a dashed stroke as an object, like the DTCG value.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(untagged)]
pub enum ResolvedStrokeStyle {
    Keyword(StrokeKeyword),
    Dashed(ResolvedDashedStroke),
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResolvedBorder {
    pub color: ResolvedColor,
    pub width: ResolvedDimension,
    pub style: ResolvedStrokeStyle,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedTransition {
    pub duration: ResolvedDuration,
    pub delay: ResolvedDuration,
    pub timing_function: ResolvedCubicBezier,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResolvedGradientStop {
    pub color: ResolvedColor,
    /// In `[0, 1]`. An alias to a number outside that range is clamped.
    pub position: f64,
}

/// A gradient that refers to other gradients has their stops spliced in, in order.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ResolvedGradient {
    pub stops: Vec<ResolvedGradientStop>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedTypography {
    pub font_family: ResolvedFontFamily,
    pub font_size: ResolvedDimension,
    pub font_weight: ResolvedFontWeight,
    pub letter_spacing: ResolvedDimension,
    /// A multiplier of the font size.
    pub line_height: ResolvedNumber,
}

/// The fully resolved `$value` of a token, one variant per DTCG type.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(tag = "type", content = "value", rename_all = "camelCase")]
pub enum ResolvedValue {
    Color(ResolvedColor),
    Dimension(ResolvedDimension),
    Number(ResolvedNumber),
    Duration(ResolvedDuration),
    CubicBezier(ResolvedCubicBezier),
    Shadow(ResolvedShadow),
    FontFamily(ResolvedFontFamily),
    FontWeight(ResolvedFontWeight),
    StrokeStyle(ResolvedStrokeStyle),
    Border(ResolvedBorder),
    Transition(ResolvedTransition),
    Gradient(ResolvedGradient),
    Typography(ResolvedTypography),
}

/// One token as a plugin sees it. The serialized form is the contract with the Node and WASM bindings, so field
/// names are camelCase and values are tagged by type.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedToken {
    /// Member names from the root, like `["color", "brand", "$root"]`.
    pub path: Vec<String>,
    pub token_type: TokenType,
    pub value: ResolvedValue,
    /// The path of the token this one is an alias of, so a backend can emit `var(--x)` instead of the literal.
    /// Only a token-level `{alias}` is recorded here, not aliases inside composite members.
    pub alias_of: Option<Vec<String>>,
    pub description: Option<String>,
    /// `Some(None)` is `"$deprecated": true`, and `Some(Some(message))` carries the reason.
    #[serde(serialize_with = "serialize_deprecated")]
    pub deprecated: Option<Option<String>>,
}

/// `false` or a missing `$deprecated` is `null`, `true` is `true`, and a message is the string.
fn serialize_deprecated<S: serde::Serializer>(
    value: &Option<Option<String>>,
    s: S,
) -> Result<S::Ok, S::Error> {
    match value {
        None => s.serialize_none(),
        Some(None) => s.serialize_bool(true),
        Some(Some(message)) => s.serialize_str(message),
    }
}

/// Resolved values by token id. Groups and invalid tokens have no entry. It serializes as an array of tokens in
/// declaration order, which is deterministic.
#[derive(Debug, Default)]
pub struct ResolvedTokens {
    pub tokens: IndexMap<TokenId, ResolvedToken>,
    /// Where each token and group sits, so aliases found deep inside a composite can still be named.
    paths: IndexMap<TokenId, Vec<String>>,
}

impl Serialize for ResolvedTokens {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_seq(self.tokens.values())
    }
}

impl ResolvedTokens {
    pub fn get(&self, id: TokenId) -> Option<&ResolvedToken> {
        self.tokens.get(&id)
    }
}

/// The path of every token and group, found by walking the groups once from the root.
fn paths(arena: &TokenArena) -> IndexMap<TokenId, Vec<String>> {
    fn walk(
        arena: &TokenArena,
        id: TokenId,
        path: &mut Vec<String>,
        out: &mut IndexMap<TokenId, Vec<String>>,
    ) {
        out.insert(id, path.clone());
        if let AnyToken::Group(group) = arena.get(id) {
            for (name, child) in &group.value.tokens {
                path.push(name.clone());
                walk(arena, *child, path, out);
                path.pop();
            }
        }
    }
    let mut out = IndexMap::new();
    walk(arena, arena.root, &mut Vec::new(), &mut out);
    out
}

pub fn resolve(arena: &TokenArena, a: &Analyzed) -> ResolvedTokens {
    let mut out = ResolvedTokens {
        paths: paths(arena),
        ..Default::default()
    };
    for (id, _) in arena.tokens() {
        resolve_token(arena, a, id, &mut out);
    }
    out
}

/// Resolves one token, following an alias into its target first. The graph is acyclic by now, so this ends.
fn resolve_token(
    arena: &TokenArena,
    a: &Analyzed,
    id: TokenId,
    out: &mut ResolvedTokens,
) -> Option<()> {
    if out.tokens.contains_key(&id) {
        return Some(());
    }
    let AnyToken::Token(token) = arena.get(id) else {
        return None;
    };
    let token = &token.value;

    let value = match &token.value.value {
        RefAliasOrValue::Alias(target) => {
            resolve_token(arena, a, *target, out)?;
            out.tokens.get(target)?.value.clone()
        }
        RefAliasOrValue::Value(v) => resolve_value(arena, a, out, v)?,
        RefAliasOrValue::Reference(end) => {
            // The end of a token-level `$ref` is the literal value, so read it as the token's own type.
            let parsed = parse_token_value(token.token_type, *end, a, &mut DiagnosticStore::new())?;
            resolve_value(arena, a, out, &parsed)?
        }
    };

    out.tokens.insert(
        id,
        ResolvedToken {
            path: out.paths.get(&id).cloned().unwrap_or_default(),
            token_type: token.token_type,
            value,
            alias_of: match &token.value.value {
                RefAliasOrValue::Alias(target) => out.paths.get(target).cloned(),
                _ => None,
            },
            description: token.common.description.as_ref().map(|d| d.value.clone()),
            deprecated: match token.common.deprecation.as_ref().map(|d| &d.value) {
                Some(Deprecation::Boolean(true)) => Some(None),
                Some(Deprecation::WithMessage(m)) => Some(Some(m.clone())),
                _ => None,
            },
        },
    );
    Some(())
}

fn resolve_value(
    arena: &TokenArena,
    a: &Analyzed,
    out: &mut ResolvedTokens,
    value: &TokenValue,
) -> Option<ResolvedValue> {
    match value {
        TokenValue::Color(c) => resolve_color(a, c).map(ResolvedValue::Color),
        TokenValue::Dimension(d) => resolve_dimension(a, d).map(ResolvedValue::Dimension),
        TokenValue::Number(n) => resolve_number(n).map(ResolvedValue::Number),
        TokenValue::Duration(d) => resolve_duration(a, d).map(ResolvedValue::Duration),
        TokenValue::CubicBezier(c) => resolve_cubic_bezier(a, c).map(ResolvedValue::CubicBezier),
        TokenValue::Shadow(sh) => resolve_shadow(arena, a, out, sh).map(ResolvedValue::Shadow),
        TokenValue::FontFamily(f) => resolve_font_family(a, f).map(ResolvedValue::FontFamily),
        TokenValue::FontWeight(w) => Some(ResolvedValue::FontWeight(ResolvedFontWeight(w.0))),
        TokenValue::StrokeStyle(st) => {
            resolve_stroke_style(arena, a, out, st).map(ResolvedValue::StrokeStyle)
        }
        TokenValue::Border(b) => resolve_border(arena, a, out, b).map(ResolvedValue::Border),
        TokenValue::Transition(t) => {
            resolve_transition(arena, a, out, t).map(ResolvedValue::Transition)
        }
        TokenValue::Gradient(g) => resolve_gradient(arena, a, out, g).map(ResolvedValue::Gradient),
        TokenValue::Typography(t) => {
            resolve_typography(arena, a, out, t).map(ResolvedValue::Typography)
        }
    }
}

/// Resolves a whole-value member of a composite. A literal is resolved in place, a `$ref` is read as `T` from the
/// literal it ends at, and an alias takes the resolved value of the token it names.
fn resolve_composite_slot<T: FromProp, R: Clone>(
    arena: &TokenArena,
    a: &Analyzed,
    out: &mut ResolvedTokens,
    slot: &ValueRefOrAlias<T>,
    resolve: fn(&TokenArena, &Analyzed, &mut ResolvedTokens, &T) -> Option<R>,
    pick: fn(&ResolvedValue) -> Option<&R>,
) -> Option<R> {
    match slot {
        ValueRefOrAlias::Value(v) => resolve(arena, a, out, v),
        ValueRefOrAlias::Reference(end) => {
            let parsed = T::from_prop(*end, a, &mut DiagnosticStore::new())?;
            resolve(arena, a, out, &parsed)
        }
        ValueRefOrAlias::Alias(target) => {
            let id = *arena.by_prop.get(target)?;
            resolve_token(arena, a, id, out)?;
            pick(&out.tokens.get(&id)?.value).cloned()
        }
    }
}

fn resolve_shadow(
    arena: &TokenArena,
    a: &Analyzed,
    out: &mut ResolvedTokens,
    shadow: &Shadow,
) -> Option<ResolvedShadow> {
    let mut layers = Vec::new();
    for entry in &shadow.layers {
        match &entry.value {
            ValueRefOrAlias::Value(layer) => {
                layers.push(resolve_shadow_layer(arena, a, out, layer)?)
            }
            ValueRefOrAlias::Reference(end) => {
                let parsed = Shadow::from_prop(*end, a, &mut DiagnosticStore::new())?;
                layers.extend(resolve_shadow(arena, a, out, &parsed)?.layers);
            }
            ValueRefOrAlias::Alias(target) => {
                let id = *arena.by_prop.get(target)?;
                resolve_token(arena, a, id, out)?;
                let ResolvedValue::Shadow(other) = &out.tokens.get(&id)?.value else {
                    return None;
                };
                layers.extend(other.layers.iter().cloned());
            }
        }
    }
    Some(ResolvedShadow { layers })
}

fn resolve_shadow_layer(
    arena: &TokenArena,
    a: &Analyzed,
    out: &mut ResolvedTokens,
    layer: &ShadowLayer,
) -> Option<ResolvedShadowLayer> {
    let dimension = |slot: &ValueRefOrAlias<Dimension>, out: &mut ResolvedTokens| {
        resolve_composite_slot(
            arena,
            a,
            out,
            slot,
            |_, a, _, d| resolve_dimension(a, d),
            |v| match v {
                ResolvedValue::Dimension(d) => Some(d),
                _ => None,
            },
        )
    };
    let offset_x = dimension(&layer.offset_x.value, out)?;
    let offset_y = dimension(&layer.offset_y.value, out)?;
    let blur = dimension(&layer.blur.value, out)?;
    let spread = dimension(&layer.spread.value, out)?;
    let color = resolve_composite_slot(
        arena,
        a,
        out,
        &layer.color.value,
        |_, a, _, c| resolve_color(a, c),
        |v| match v {
            ResolvedValue::Color(c) => Some(c),
            _ => None,
        },
    )?;
    let inset = match &layer.inset {
        None => false,
        Some(n) => match n.value {
            ValueOrRef::Value(b) => b,
            ValueOrRef::Reference(end) => match a.syntax(end) {
                DtcgValueSyntax::Bool(b) => *b,
                _ => return None,
            },
        },
    };
    Some(ResolvedShadowLayer {
        color,
        offset_x,
        offset_y,
        blur,
        spread,
        inset,
    })
}

fn resolve_color(a: &Analyzed, c: &Color) -> Option<ResolvedColor> {
    let color_space = match c.color_space.value {
        ValueOrRef::Value(s) => s,
        ValueOrRef::Reference(end) => ColorSpace::parse(a.string(end)?)?,
    };
    let component = |c: Component| match c {
        Component::Number(n) => Some(n),
        Component::None => None,
    };
    let components = match &c.components.value {
        ValueOrRef::Value(parts) => {
            let mut out = [None; 3];
            for (slot, part) in out.iter_mut().zip(parts) {
                *slot = match part.value {
                    ValueOrRef::Value(v) => component(v),
                    ValueOrRef::Reference(end) => component(read_component(a, end)?),
                };
            }
            out
        }
        ValueOrRef::Reference(end) => {
            let mut out = [None; 3];
            for (slot, &item) in out.iter_mut().zip(a.items(*end)?) {
                *slot = component(read_component(a, item)?);
            }
            out
        }
    };
    let alpha = match &c.alpha {
        None => 1.0,
        Some(n) => match n.value {
            ValueOrRef::Value(v) => v,
            ValueOrRef::Reference(end) => a.number(end)?,
        },
    };
    let hex = match &c.hex {
        None => None,
        Some(n) => Some(match &n.value {
            ValueOrRef::Value(v) => v.clone(),
            ValueOrRef::Reference(end) => a.string(*end)?.to_string(),
        }),
    };
    Some(ResolvedColor {
        color_space,
        components,
        alpha,
        hex,
    })
}

fn resolve_dimension(a: &Analyzed, d: &Dimension) -> Option<ResolvedDimension> {
    let value = match d.value.value {
        ValueOrRef::Value(n) => n,
        ValueOrRef::Reference(end) => a.number(end)?,
    };
    let unit = match d.unit.value {
        ValueOrRef::Value(u) => u,
        ValueOrRef::Reference(end) => DimensionUnit::parse(a.string(end)?)?,
    };
    Some(ResolvedDimension { value, unit })
}

/// One reader per member type, so the composites below read as a list of members.
macro_rules! slot_reader {
    ($name:ident, $ty:ty, $variant:ident, $resolved:ty, $resolve:expr) => {
        fn $name(
            arena: &TokenArena,
            a: &Analyzed,
            out: &mut ResolvedTokens,
            slot: &ValueRefOrAlias<$ty>,
        ) -> Option<$resolved> {
            resolve_composite_slot(arena, a, out, slot, $resolve, |v| match v {
                ResolvedValue::$variant(r) => Some(r),
                _ => None,
            })
        }
    };
}

slot_reader!(slot_color, Color, Color, ResolvedColor, |_, a, _, c| {
    resolve_color(a, c)
});
slot_reader!(
    slot_dimension,
    Dimension,
    Dimension,
    ResolvedDimension,
    |_, a, _, d| { resolve_dimension(a, d) }
);
slot_reader!(
    slot_duration,
    Duration,
    Duration,
    ResolvedDuration,
    |_, a, _, d| { resolve_duration(a, d) }
);
slot_reader!(
    slot_cubic_bezier,
    CubicBezier,
    CubicBezier,
    ResolvedCubicBezier,
    |_, a, _, c| { resolve_cubic_bezier(a, c) }
);
slot_reader!(slot_number, Number, Number, ResolvedNumber, |_, _, _, n| {
    resolve_number(n)
});
slot_reader!(
    slot_font_family,
    FontFamily,
    FontFamily,
    ResolvedFontFamily,
    |_, a, _, f| { resolve_font_family(a, f) }
);
slot_reader!(
    slot_font_weight,
    FontWeight,
    FontWeight,
    ResolvedFontWeight,
    |_, _, _, w| { Some(ResolvedFontWeight(w.0)) }
);
slot_reader!(
    slot_stroke_style,
    StrokeStyle,
    StrokeStyle,
    ResolvedStrokeStyle,
    |arena, a, out, s| { resolve_stroke_style(arena, a, out, s) }
);

fn resolve_font_family(a: &Analyzed, f: &FontFamily) -> Option<ResolvedFontFamily> {
    let names = f
        .names
        .iter()
        .map(|n| match &n.value {
            ValueOrRef::Value(s) => Some(s.clone()),
            ValueOrRef::Reference(end) => a.string(*end).map(str::to_string),
        })
        .collect::<Option<_>>()?;
    Some(ResolvedFontFamily(names))
}

fn resolve_stroke_style(
    arena: &TokenArena,
    a: &Analyzed,
    out: &mut ResolvedTokens,
    style: &StrokeStyle,
) -> Option<ResolvedStrokeStyle> {
    let d = match style {
        StrokeStyle::Keyword(k) => return Some(ResolvedStrokeStyle::Keyword(*k)),
        StrokeStyle::Dashed(d) => d,
    };

    let items = match &d.dash_array.value {
        ValueOrRef::Value(items) => items.clone(),
        ValueOrRef::Reference(end) => dash_array_items(a, *end, &mut DiagnosticStore::new())?,
    };
    let dash_array = items
        .iter()
        .map(|item| slot_dimension(arena, a, out, &item.value))
        .collect::<Option<_>>()?;
    let line_cap = match d.line_cap.value {
        ValueOrRef::Value(c) => c,
        ValueOrRef::Reference(end) => LineCap::parse(a.string(end)?)?,
    };
    Some(ResolvedStrokeStyle::Dashed(ResolvedDashedStroke {
        dash_array,
        line_cap,
    }))
}

fn resolve_border(
    arena: &TokenArena,
    a: &Analyzed,
    out: &mut ResolvedTokens,
    b: &Border,
) -> Option<ResolvedBorder> {
    Some(ResolvedBorder {
        color: slot_color(arena, a, out, &b.color.value)?,
        width: slot_dimension(arena, a, out, &b.width.value)?,
        style: slot_stroke_style(arena, a, out, &b.style.value)?,
    })
}

fn resolve_transition(
    arena: &TokenArena,
    a: &Analyzed,
    out: &mut ResolvedTokens,
    t: &Transition,
) -> Option<ResolvedTransition> {
    Some(ResolvedTransition {
        duration: slot_duration(arena, a, out, &t.duration.value)?,
        delay: slot_duration(arena, a, out, &t.delay.value)?,
        timing_function: slot_cubic_bezier(arena, a, out, &t.timing_function.value)?,
    })
}

fn resolve_typography(
    arena: &TokenArena,
    a: &Analyzed,
    out: &mut ResolvedTokens,
    t: &Typography,
) -> Option<ResolvedTypography> {
    Some(ResolvedTypography {
        font_family: slot_font_family(arena, a, out, &t.font_family.value)?,
        font_size: slot_dimension(arena, a, out, &t.font_size.value)?,
        font_weight: slot_font_weight(arena, a, out, &t.font_weight.value)?,
        letter_spacing: slot_dimension(arena, a, out, &t.letter_spacing.value)?,
        line_height: slot_number(arena, a, out, &t.line_height.value)?,
    })
}

fn resolve_gradient(
    arena: &TokenArena,
    a: &Analyzed,
    out: &mut ResolvedTokens,
    gradient: &Gradient,
) -> Option<ResolvedGradient> {
    let mut stops = Vec::new();
    for entry in &gradient.stops {
        match &entry.value {
            ValueRefOrAlias::Value(stop) => stops.push(resolve_gradient_stop(arena, a, out, stop)?),
            ValueRefOrAlias::Reference(end) => {
                let parsed = Gradient::from_prop(*end, a, &mut DiagnosticStore::new())?;
                stops.extend(resolve_gradient(arena, a, out, &parsed)?.stops);
            }
            ValueRefOrAlias::Alias(target) => {
                let id = *arena.by_prop.get(target)?;
                resolve_token(arena, a, id, out)?;
                let ResolvedValue::Gradient(other) = &out.tokens.get(&id)?.value else {
                    return None;
                };
                stops.extend(other.stops.iter().cloned());
            }
        }
    }
    Some(ResolvedGradient { stops })
}

fn resolve_gradient_stop(
    arena: &TokenArena,
    a: &Analyzed,
    out: &mut ResolvedTokens,
    stop: &GradientStop,
) -> Option<ResolvedGradientStop> {
    let ResolvedNumber(position) = slot_number(arena, a, out, &stop.position.value)?;
    Some(ResolvedGradientStop {
        color: slot_color(arena, a, out, &stop.color.value)?,
        position: position.clamp(0.0, 1.0),
    })
}

fn resolve_cubic_bezier(a: &Analyzed, c: &CubicBezier) -> Option<ResolvedCubicBezier> {
    let mut out = [0.0; 4];
    match &c.0.value {
        ValueOrRef::Value(items) => {
            for (slot, item) in out.iter_mut().zip(items) {
                *slot = match item.value {
                    ValueOrRef::Value(n) => n,
                    ValueOrRef::Reference(end) => a.number(end)?,
                };
            }
        }
        ValueOrRef::Reference(end) => {
            for (slot, &item) in out.iter_mut().zip(a.items(*end)?) {
                *slot = a.number(item)?;
            }
        }
    }
    Some(ResolvedCubicBezier(out))
}

fn resolve_number(n: &Number) -> Option<ResolvedNumber> {
    Some(ResolvedNumber(n.0))
}

fn resolve_duration(a: &Analyzed, d: &Duration) -> Option<ResolvedDuration> {
    let value = match d.value.value {
        ValueOrRef::Value(n) => n,
        ValueOrRef::Reference(end) => a.number(end)?,
    };
    let unit = match d.unit.value {
        ValueOrRef::Value(u) => u,
        ValueOrRef::Reference(end) => DurationUnit::parse(a.string(end)?)?,
    };
    Some(ResolvedDuration { value, unit })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ir::{
            merge::LayerOrigin,
            pipeline::{Compiled, Layer, compile},
        },
        source::SourceIdGenerator,
    };

    fn compiled(doc: &str) -> Compiled {
        let (tree, errors) = json_spanned::parse_json(doc);
        assert!(errors.is_empty());
        let tree = tree.unwrap();
        let source = SourceIdGenerator::default().generate();
        let layers = [Layer {
            source,
            origin: LayerOrigin::File(source),
            tree: &tree,
        }];
        let mut diags = DiagnosticStore::new();
        let c = compile(&layers, &mut diags).ok().expect("not blocked");
        assert!(!diags.has_errors(), "{:?}", diags.diagnostics());
        c
    }

    fn dim(c: &Compiled, r: &ResolvedTokens, path: &[&str]) -> ResolvedDimension {
        let id = c.arena.find(path).expect("exists");
        match &r.get(id).expect("resolved").value {
            ResolvedValue::Dimension(d) => d.clone(),
            other => panic!("not a dimension: {other:?}"),
        }
    }

    fn color(c: &Compiled, r: &ResolvedTokens, path: &[&str]) -> ResolvedColor {
        let id = c.arena.find(path).expect("exists");
        match &r.get(id).expect("resolved").value {
            ResolvedValue::Color(c) => c.clone(),
            other => panic!("not a color: {other:?}"),
        }
    }

    #[test]
    fn aliases_slot_refs_and_token_refs_all_resolve_to_literals() {
        let c = compiled(
            r##"{
            "$type": "dimension",
            "a": {"$value": {"value": 4, "unit": "px"}},
            "alias": {"$value": "{a}"},
            "alias2": {"$value": "{alias}"},
            "slot": {"$value": {"value": {"$ref": "#/a/$value/value"}, "unit": {"$ref": "#/a/$value/unit"}}},
            "tok": {"$ref": "#/a/$value"},
            "viaalias": {"$ref": "#/alias/$value"}
        }"##,
        );
        let r = c.resolved();
        let expected = ResolvedDimension {
            value: 4.0,
            unit: DimensionUnit::Px,
        };
        for name in ["a", "alias", "alias2", "slot", "tok", "viaalias"] {
            assert_eq!(dim(&c, &r, &[name]), expected, "{name}");
        }
    }

    #[test]
    fn invalid_tokens_and_groups_have_no_entry() {
        let c = compiled(
            r#"{"$type": "dimension", "g": {"x": {"$value": {"value": 1, "unit": "rem"}}}}"#,
        );
        let r = c.resolved();
        assert_eq!(r.tokens.len(), 1);
        assert!(r.get(c.arena.find(&["g"]).unwrap()).is_none());
    }

    #[test]
    fn colors_resolve_every_reference_and_default_the_alpha() {
        let c = compiled(
            r##"{
            "$type": "color",
            "base": {"$value": {"colorSpace": "hsl", "components": [200, "none", 50], "hex": "#336699"}},
            "alias": {"$value": "{base}"},
            "tok": {"$ref": "#/base/$value"},
            "n": {"$value": {"colorSpace": "srgb", "components": [0.25, 0, 0], "alpha": 0.25}},
            "mixed": {"$value": {
                "colorSpace": "srgb",
                "components": [{"$ref": "#/n/$value/components/0"}, 0, "none"],
                "alpha": {"$ref": "#/n/$value/alpha"}
            }},
            "whole": {"$value": {"colorSpace": "srgb", "components": {"$ref": "#/arr/$value/components"}}},
            "arr": {"$value": {"colorSpace": "srgb", "components": [1, 0.5, 0]}}
        }"##,
        );
        let r = c.resolved();
        let base = ResolvedColor {
            color_space: ColorSpace::Hsl,
            components: [Some(200.0), None, Some(50.0)],
            alpha: 1.0,
            hex: Some("#336699".into()),
        };
        for name in ["base", "alias", "tok"] {
            assert_eq!(color(&c, &r, &[name]), base, "{name}");
        }
        let mixed = color(&c, &r, &["mixed"]);
        assert_eq!(mixed.components, [Some(0.25), Some(0.0), None]);
        assert_eq!(mixed.alpha, 0.25);
        assert_eq!(
            color(&c, &r, &["whole"]).components,
            [Some(1.0), Some(0.5), Some(0.0)]
        );
    }

    fn number(c: &Compiled, r: &ResolvedTokens, path: &[&str]) -> f64 {
        let id = c.arena.find(path).expect("exists");
        match &r.get(id).expect("resolved").value {
            ResolvedValue::Number(n) => n.0,
            other => panic!("not a number: {other:?}"),
        }
    }

    #[test]
    fn numbers_resolve_through_aliases_and_token_refs() {
        let c = compiled(
            r##"{
            "$type": "number",
            "a": {"$value": 1.5},
            "alias": {"$value": "{a}"},
            "alias2": {"$value": "{alias}"},
            "tok": {"$ref": "#/a/$value"},
            "viaalias": {"$ref": "#/alias/$value"},
            "zero": {"$value": 0},
            "neg": {"$value": -3}
        }"##,
        );
        let r = c.resolved();
        for name in ["a", "alias", "alias2", "tok", "viaalias"] {
            assert_eq!(number(&c, &r, &[name]), 1.5, "{name}");
        }
        assert_eq!(number(&c, &r, &["zero"]), 0.0);
        assert_eq!(number(&c, &r, &["neg"]), -3.0);
        assert!(r.get(c.arena.find(&[]).unwrap()).is_none());
    }

    #[test]
    fn a_number_can_be_inside_another_types_slot() {
        let c = compiled(
            r##"{
            "n": {"$type": "number", "$value": 8},
            "d": {"$type": "dimension", "$value": {"value": {"$ref": "#/n/$value"}, "unit": "px"}}
        }"##,
        );
        let r = c.resolved();
        assert_eq!(number(&c, &r, &["n"]), 8.0);
        assert_eq!(dim(&c, &r, &["d"]).value, 8.0);
    }

    fn duration(c: &Compiled, r: &ResolvedTokens, path: &[&str]) -> ResolvedDuration {
        let id = c.arena.find(path).expect("exists");
        match &r.get(id).expect("resolved").value {
            ResolvedValue::Duration(d) => d.clone(),
            other => panic!("not a duration: {other:?}"),
        }
    }

    #[test]
    fn durations_resolve_aliases_slot_refs_and_token_refs() {
        let c = compiled(
            r##"{
            "$type": "duration",
            "a": {"$value": {"value": 200, "unit": "ms"}},
            "alias": {"$value": "{a}"},
            "alias2": {"$value": "{alias}"},
            "slot": {"$value": {"value": {"$ref": "#/a/$value/value"}, "unit": {"$ref": "#/a/$value/unit"}}},
            "tok": {"$ref": "#/a/$value"},
            "viaalias": {"$ref": "#/alias/$value"}
        }"##,
        );
        let r = c.resolved();
        let expected = ResolvedDuration {
            value: 200.0,
            unit: DurationUnit::Ms,
        };
        for name in ["a", "alias", "alias2", "slot", "tok", "viaalias"] {
            assert_eq!(duration(&c, &r, &[name]), expected, "{name}");
        }
    }

    #[test]
    fn a_duration_value_can_come_from_a_number_token() {
        let c = compiled(
            r##"{
            "n": {"$type": "number", "$value": 0.3},
            "d": {"$type": "duration", "$value": {"value": {"$ref": "#/n/$value"}, "unit": "s"}}
        }"##,
        );
        let r = c.resolved();
        assert_eq!(
            duration(&c, &r, &["d"]),
            ResolvedDuration {
                value: 0.3,
                unit: DurationUnit::S
            }
        );
    }

    const RED: &str = r#"{"colorSpace": "srgb", "components": [1, 0, 0]}"#;

    fn red() -> ResolvedColor {
        ResolvedColor {
            color_space: ColorSpace::Srgb,
            components: [Some(1.0), Some(0.0), Some(0.0)],
            alpha: 1.0,
            hex: None,
        }
    }

    fn px(n: f64) -> ResolvedDimension {
        ResolvedDimension {
            value: n,
            unit: DimensionUnit::Px,
        }
    }

    fn shadow(c: &Compiled, r: &ResolvedTokens, path: &[&str]) -> ResolvedShadow {
        let id = c.arena.find(path).expect("exists");
        match &r.get(id).expect("resolved").value {
            ResolvedValue::Shadow(s) => s.clone(),
            other => panic!("not a shadow: {other:?}"),
        }
    }

    #[test]
    fn a_literal_shadow_resolves_and_inset_defaults_to_false() {
        let c = compiled(&format!(
            r##"{{"s": {{"$type": "shadow", "$value": {{
                "color": {RED}, "offsetX": {{"value": 1, "unit": "px"}}, "offsetY": {{"value": 2, "unit": "px"}},
                "blur": {{"value": 3, "unit": "px"}}, "spread": {{"value": 4, "unit": "px"}}
            }}}}}}"##
        ));
        let sh = shadow(&c, &c.resolved(), &["s"]);
        assert_eq!(
            sh.layers,
            [ResolvedShadowLayer {
                color: red(),
                offset_x: px(1.0),
                offset_y: px(2.0),
                blur: px(3.0),
                spread: px(4.0),
                inset: false,
            }]
        );
    }

    #[test]
    fn every_way_of_naming_a_member_resolves_to_the_same_shadow() {
        let c = compiled(&format!(
            r##"{{
            "red": {{"$type": "color", "$value": {RED}}},
            "two": {{"$type": "dimension", "$value": {{"value": 2, "unit": "px"}}}},
            "literal": {{"$type": "shadow", "$value": {{
                "color": {RED}, "offsetX": {{"value": 2, "unit": "px"}}, "offsetY": {{"value": 2, "unit": "px"}},
                "blur": {{"value": 2, "unit": "px"}}, "spread": {{"value": 2, "unit": "px"}}, "inset": true
            }}}},
            "aliased": {{"$type": "shadow", "$value": {{
                "color": "{{red}}", "offsetX": "{{two}}", "offsetY": "{{two}}", "blur": "{{two}}", "spread": "{{two}}",
                "inset": {{"$ref": "#/literal/$value/inset"}}
            }}}},
            "referenced": {{"$type": "shadow", "$value": {{
                "color": {{"$ref": "#/red/$value"}}, "offsetX": {{"$ref": "#/two/$value"}},
                "offsetY": {{"$ref": "#/two/$value"}}, "blur": {{"$ref": "#/two/$value"}},
                "spread": {{"$ref": "#/two/$value"}}, "inset": true
            }}}},
            "whole_alias": {{"$type": "shadow", "$value": "{{literal}}"}},
            "whole_ref": {{"$type": "shadow", "$ref": "#/aliased/$value"}}
        }}"##
        ));
        let r = c.resolved();
        let expected = shadow(&c, &r, &["literal"]);
        assert!(expected.layers[0].inset);
        for name in ["aliased", "referenced", "whole_alias", "whole_ref"] {
            assert_eq!(shadow(&c, &r, &[name]), expected, "{name}");
        }
    }

    #[test]
    fn a_member_alias_follows_a_chain_of_aliases() {
        let c = compiled(&format!(
            r##"{{
            "$type": "dimension",
            "base": {{"$value": {{"value": 5, "unit": "px"}}}},
            "mid": {{"$value": "{{base}}"}},
            "s": {{"$type": "shadow", "$value": {{
                "color": {RED}, "offsetX": "{{mid}}", "offsetY": "{{mid}}", "blur": "{{mid}}", "spread": "{{mid}}"
            }}}}
        }}"##
        ));
        let sh = shadow(&c, &c.resolved(), &["s"]);
        assert_eq!(sh.layers[0].offset_x, px(5.0));
    }

    #[test]
    fn a_shadow_list_flattens_references_to_other_shadows_in_order() {
        let c = compiled(&format!(
            r##"{{
            "$type": "shadow",
            "a": {{"$value": {{"color": {RED}, "offsetX": {{"value": 1, "unit": "px"}}, "offsetY": {{"value": 1, "unit": "px"}},
                "blur": {{"value": 1, "unit": "px"}}, "spread": {{"value": 1, "unit": "px"}}}}}},
            "b": {{"$value": {{"color": {RED}, "offsetX": {{"value": 2, "unit": "px"}}, "offsetY": {{"value": 2, "unit": "px"}},
                "blur": {{"value": 2, "unit": "px"}}, "spread": {{"value": 2, "unit": "px"}}, "inset": true}}}},
            "pair": {{"$value": ["{{a}}", "{{b}}"]}},
            "all": {{"$value": [
                {{"color": {RED}, "offsetX": {{"value": 9, "unit": "px"}}, "offsetY": {{"value": 9, "unit": "px"}},
                  "blur": {{"value": 9, "unit": "px"}}, "spread": {{"value": 9, "unit": "px"}}}},
                "{{pair}}",
                {{"$ref": "#/a/$value"}}
            ]}}
        }}"##
        ));
        let r = c.resolved();
        let offsets: Vec<_> = shadow(&c, &r, &["pair"])
            .layers
            .iter()
            .map(|l| l.offset_x.value)
            .collect();
        assert_eq!(offsets, [1.0, 2.0]);
        let all = shadow(&c, &r, &["all"]);
        let offsets: Vec<_> = all.layers.iter().map(|l| l.offset_x.value).collect();
        assert_eq!(offsets, [9.0, 1.0, 2.0, 1.0]);
        assert_eq!(
            all.layers.iter().map(|l| l.inset).collect::<Vec<_>>(),
            [false, false, true, false]
        );
    }

    fn bezier(c: &Compiled, r: &ResolvedTokens, path: &[&str]) -> [f64; 4] {
        let id = c.arena.find(path).expect("exists");
        match &r.get(id).expect("resolved").value {
            ResolvedValue::CubicBezier(b) => b.0,
            other => panic!("not a cubic bezier: {other:?}"),
        }
    }

    #[test]
    fn cubic_beziers_resolve_through_every_kind_of_reference() {
        let c = compiled(
            r##"{
            "n": {"$type": "number", "$value": 0.25},
            "$type": "cubicBezier",
            "a": {"$value": [0.1, 0.2, 0.3, 0.4]},
            "alias": {"$value": "{a}"},
            "alias2": {"$value": "{alias}"},
            "tok": {"$ref": "#/a/$value"},
            "whole": {"$value": {"$ref": "#/a/$value"}},
            "items": {"$value": [{"$ref": "#/n/$value"}, {"$ref": "#/n/$value"}, 0.5, -1]}
        }"##,
        );
        let r = c.resolved();
        for name in ["a", "alias", "alias2", "tok", "whole"] {
            assert_eq!(bezier(&c, &r, &[name]), [0.1, 0.2, 0.3, 0.4], "{name}");
        }
        assert_eq!(bezier(&c, &r, &["items"]), [0.25, 0.25, 0.5, -1.0]);
    }

    #[test]
    fn serialized_form_is_the_plugin_contract() {
        let c = compiled(
            r##"{
            "space": {
                "$type": "dimension",
                "sm": {"$value": {"value": 4, "unit": "px"}, "$description": "small", "$deprecated": "use md"},
                "alias": {"$value": "{space.sm}", "$deprecated": true}
            },
            "brand": {"$type": "color", "$value": {"colorSpace": "display-p3", "components": [1, "none", 0.5]}},
            "fast": {"$type": "duration", "$value": {"value": 100, "unit": "ms"}}
        }"##,
        );
        let json = serde_json::to_value(c.resolved()).unwrap();
        assert_eq!(
            json,
            serde_json::json!([
                {
                    "path": ["space", "sm"], "tokenType": "dimension",
                    "value": {"type": "dimension", "value": {"value": 4.0, "unit": "px"}},
                    "aliasOf": null, "description": "small", "deprecated": "use md"
                },
                {
                    "path": ["space", "alias"], "tokenType": "dimension",
                    "value": {"type": "dimension", "value": {"value": 4.0, "unit": "px"}},
                    "aliasOf": ["space", "sm"], "description": null, "deprecated": true
                },
                {
                    "path": ["brand"], "tokenType": "color",
                    "value": {"type": "color", "value": {
                        "colorSpace": "display-p3", "components": [1.0, null, 0.5], "alpha": 1.0, "hex": null
                    }},
                    "aliasOf": null, "description": null, "deprecated": null
                },
                {
                    "path": ["fast"], "tokenType": "duration",
                    "value": {"type": "duration", "value": {"value": 100.0, "unit": "ms"}},
                    "aliasOf": null, "description": null, "deprecated": null
                }
            ])
        );
    }

    fn json_of(c: &Compiled, path: &[&str]) -> serde_json::Value {
        let r = c.resolved();
        let id = c.arena.find(path).expect("exists");
        serde_json::to_value(&r.get(id).expect("resolved").value).unwrap()
    }

    #[test]
    fn font_families_and_weights_resolve_through_every_kind_of_reference() {
        let c = compiled(
            r##"{
            "family": {"$type": "fontFamily", "$value": ["Inter", "sans-serif"]},
            "alias": {"$type": "fontFamily", "$value": "{family}"},
            "tok": {"$type": "fontFamily", "$ref": "#/family/$value"},
            "name": {"$type": "fontFamily", "$value": [{"$ref": "#/family/$value/0"}, "serif"]},
            "single": {"$type": "fontFamily", "$value": "Georgia"},
            "weight": {"$type": "fontWeight", "$value": "semi-bold"},
            "weightAlias": {"$type": "fontWeight", "$value": "{weight}"},
            "numeric": {"$type": "fontWeight", "$value": 350}
        }"##,
        );
        let fam = |p: &str| json_of(&c, &[p]);
        let both = serde_json::json!({"type": "fontFamily", "value": ["Inter", "sans-serif"]});
        assert_eq!(fam("family"), both);
        assert_eq!(fam("alias"), both);
        assert_eq!(fam("tok"), both);
        assert_eq!(fam("name")["value"], serde_json::json!(["Inter", "serif"]));
        assert_eq!(fam("single")["value"], serde_json::json!(["Georgia"]));
        assert_eq!(fam("weight")["value"], 600.0);
        assert_eq!(fam("weightAlias")["value"], 600.0);
        assert_eq!(fam("numeric")["value"], 350.0);
    }

    #[test]
    fn stroke_styles_serialize_like_the_dtcg_value() {
        let c = compiled(
            r##"{
            "gap": {"$type": "dimension", "$value": {"value": 3, "unit": "px"}},
            "kw": {"$type": "strokeStyle", "$value": "dotted"},
            "kwAlias": {"$type": "strokeStyle", "$value": "{kw}"},
            "dashed": {"$type": "strokeStyle", "$value": {
                "dashArray": [{"value": 4, "unit": "px"}, "{gap}", {"$ref": "#/gap/$value"}],
                "lineCap": "round"
            }},
            "whole": {"$type": "strokeStyle", "$value": {
                "dashArray": {"$ref": "#/dashed/$value/dashArray"},
                "lineCap": {"$ref": "#/dashed/$value/lineCap"}
            }}
        }"##,
        );
        assert_eq!(json_of(&c, &["kw"])["value"], "dotted");
        assert_eq!(json_of(&c, &["kwAlias"])["value"], "dotted");
        let expected = serde_json::json!({
            "dashArray": [
                {"value": 4.0, "unit": "px"}, {"value": 3.0, "unit": "px"}, {"value": 3.0, "unit": "px"}
            ],
            "lineCap": "round"
        });
        assert_eq!(json_of(&c, &["dashed"])["value"], expected);
        assert_eq!(json_of(&c, &["whole"])["value"], expected);
    }

    #[test]
    fn borders_transitions_and_typography_resolve_their_members() {
        let c = compiled(&format!(
            r##"{{
            "red": {{"$type": "color", "$value": {RED}}},
            "px": {{"$type": "dimension", "$value": {{"value": 2, "unit": "px"}}}},
            "dashed": {{"$type": "strokeStyle", "$value": "dashed"}},
            "fast": {{"$type": "duration", "$value": {{"value": 100, "unit": "ms"}}}},
            "ease": {{"$type": "cubicBezier", "$value": [0.25, 0.1, 0.25, 1]}},
            "family": {{"$type": "fontFamily", "$value": "Inter"}},
            "lh": {{"$type": "number", "$value": 1.5}},
            "border": {{"$type": "border", "$value": {{
                "color": "{{red}}", "width": {{"$ref": "#/px/$value"}}, "style": "{{dashed}}"
            }}}},
            "transition": {{"$type": "transition", "$value": {{
                "duration": "{{fast}}", "delay": {{"value": 0, "unit": "s"}}, "timingFunction": "{{ease}}"
            }}}},
            "type": {{"$type": "typography", "$value": {{
                "fontFamily": "{{family}}", "fontSize": "{{px}}", "fontWeight": "bold",
                "letterSpacing": {{"value": 0, "unit": "px"}}, "lineHeight": "{{lh}}"
            }}}}
        }}"##
        ));

        assert_eq!(
            json_of(&c, &["border"])["value"],
            serde_json::json!({
                "color": {"colorSpace": "srgb", "components": [1.0, 0.0, 0.0], "alpha": 1.0, "hex": null},
                "width": {"value": 2.0, "unit": "px"},
                "style": "dashed"
            })
        );
        assert_eq!(
            json_of(&c, &["transition"])["value"],
            serde_json::json!({
                "duration": {"value": 100.0, "unit": "ms"},
                "delay": {"value": 0.0, "unit": "s"},
                "timingFunction": [0.25, 0.1, 0.25, 1.0]
            })
        );
        assert_eq!(
            json_of(&c, &["type"])["value"],
            serde_json::json!({
                "fontFamily": ["Inter"],
                "fontSize": {"value": 2.0, "unit": "px"},
                "fontWeight": 700.0,
                "letterSpacing": {"value": 0.0, "unit": "px"},
                "lineHeight": 1.5
            })
        );
        assert_eq!(json_of(&c, &["type"])["type"], "typography");
    }

    #[test]
    fn a_border_style_alias_to_a_dashed_stroke_resolves() {
        let c = compiled(&format!(
            r##"{{
            "d": {{"$type": "strokeStyle", "$value": {{"dashArray": [{{"value": 1, "unit": "px"}}], "lineCap": "butt"}}}},
            "b": {{"$type": "border", "$value": {{"color": {RED}, "width": {{"value": 1, "unit": "px"}}, "style": "{{d}}"}}}}
        }}"##
        ));
        assert_eq!(
            json_of(&c, &["b"])["value"]["style"],
            serde_json::json!({"dashArray": [{"value": 1.0, "unit": "px"}], "lineCap": "butt"})
        );
    }

    #[test]
    fn gradients_splice_referenced_gradients_and_clamp_alias_positions() {
        let c = compiled(&format!(
            r##"{{
            "$type": "gradient",
            "pos": {{"$type": "number", "$value": 7}},
            "base": {{"$value": [{{"color": {RED}, "position": 0}}]}},
            "alias": {{"$value": ["{{base}}", {{"color": {RED}, "position": 1}}]}},
            "tok": {{"$ref": "#/base/$value"}},
            "nested": {{"$value": [{{"$ref": "#/base/$value"}}, "{{alias}}", {{"color": {RED}, "position": "{{pos}}"}}]}}
        }}"##
        ));
        let positions = |p: &str| -> Vec<f64> {
            json_of(&c, &[p])["value"]["stops"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s["position"].as_f64().unwrap())
                .collect()
        };
        assert_eq!(positions("base"), [0.0]);
        assert_eq!(positions("tok"), [0.0]);
        assert_eq!(positions("alias"), [0.0, 1.0]);
        // base, then alias (base + one), then the aliased 7 clamped to 1.
        assert_eq!(positions("nested"), [0.0, 0.0, 1.0, 1.0]);
    }
}
