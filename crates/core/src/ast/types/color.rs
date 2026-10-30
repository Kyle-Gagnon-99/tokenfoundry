//! The `color` module provides the AST representation of color types in DTCG (Color module, 2025.10).
//!
//! The value is an object with a required `colorSpace` and `components`, an optional `alpha` (0 to 1, assumed 1 when
//! omitted) and an optional `hex` fallback in 6 digit notation. No other members are allowed.
//!
//! `components` is always three entries, each a number or the keyword `"none"`, and what they mean and which numbers
//! are allowed depends on the color space (for example `hsl` is a hue in `[0, 360)` and two percentages). Every
//! property, and every single component, can be a `{"$ref": ...}` object instead of a literal. A range is checked on
//! the number a reference ends at, so a reference cannot sneak in an out-of-range value. When the color space itself
//! could not be determined (an unresolved `$ref`), ranges are not checked, since that has already been reported.

use crate::{
    ast::{
        node::{Node, PropId},
        types::FromProp,
        value::{
            SlotKind, ValueOrRef, alias_help, checked_reference, choice_slot, classify_slot, node,
            number_slot,
        },
    },
    diagnostics::{
        code::DiagnosticCode,
        diagnostic::{Diagnostic, DiagnosticSink},
    },
    ir::pipeline::Analyzed,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ColorSpace {
    Srgb,
    SrgbLinear,
    Hsl,
    Hwb,
    Lab,
    Lch,
    Oklab,
    Oklch,
    DisplayP3,
    #[serde(rename = "a98-rgb")]
    A98Rgb,
    ProphotoRgb,
    #[serde(rename = "rec2020")]
    Rec2020,
    XyzD65,
    XyzD50,
}

/// What numbers a component accepts.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Range {
    /// `[0, 1]`
    Unit,
    /// `[0, 100]`
    Percent,
    /// `[0, 360)`
    Hue,
    /// `[0, infinity)`
    Chroma,
    /// Any number.
    Unbounded,
}

impl Range {
    fn contains(self, n: f64) -> bool {
        match self {
            Range::Unit => (0.0..=1.0).contains(&n),
            Range::Percent => (0.0..=100.0).contains(&n),
            Range::Hue => (0.0..360.0).contains(&n),
            Range::Chroma => n >= 0.0,
            Range::Unbounded => true,
        }
    }

    fn describe(self) -> &'static str {
        match self {
            Range::Unit => "between 0 and 1",
            Range::Percent => "between 0 and 100",
            Range::Hue => "at least 0 and below 360",
            Range::Chroma => "0 or more",
            Range::Unbounded => "any number",
        }
    }
}

impl ColorSpace {
    pub(crate) const NAMES: [&'static str; 14] = [
        "srgb",
        "srgb-linear",
        "hsl",
        "hwb",
        "lab",
        "lch",
        "oklab",
        "oklch",
        "display-p3",
        "a98-rgb",
        "prophoto-rgb",
        "rec2020",
        "xyz-d65",
        "xyz-d50",
    ];

    pub(crate) fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "srgb" => Self::Srgb,
            "srgb-linear" => Self::SrgbLinear,
            "hsl" => Self::Hsl,
            "hwb" => Self::Hwb,
            "lab" => Self::Lab,
            "lch" => Self::Lch,
            "oklab" => Self::Oklab,
            "oklch" => Self::Oklch,
            "display-p3" => Self::DisplayP3,
            "a98-rgb" => Self::A98Rgb,
            "prophoto-rgb" => Self::ProphotoRgb,
            "rec2020" => Self::Rec2020,
            "xyz-d65" => Self::XyzD65,
            "xyz-d50" => Self::XyzD50,
            _ => return None,
        })
    }

    pub fn as_str(self) -> &'static str {
        Self::NAMES[self as usize]
    }

    fn ranges(self) -> [Range; 3] {
        use Range::*;
        match self {
            Self::Srgb
            | Self::SrgbLinear
            | Self::DisplayP3
            | Self::A98Rgb
            | Self::ProphotoRgb
            | Self::Rec2020
            | Self::XyzD65
            | Self::XyzD50 => [Unit; 3],
            Self::Hsl | Self::Hwb => [Hue, Percent, Percent],
            Self::Lab => [Percent, Unbounded, Unbounded],
            Self::Lch => [Percent, Chroma, Hue],
            Self::Oklab => [Unit, Unbounded, Unbounded],
            Self::Oklch => [Unit, Chroma, Hue],
        }
    }

    fn component_names(self) -> [&'static str; 3] {
        match self {
            Self::Srgb
            | Self::SrgbLinear
            | Self::DisplayP3
            | Self::A98Rgb
            | Self::ProphotoRgb
            | Self::Rec2020 => ["red", "green", "blue"],
            Self::XyzD65 | Self::XyzD50 => ["x", "y", "z"],
            Self::Hsl => ["hue", "saturation", "lightness"],
            Self::Hwb => ["hue", "whiteness", "blackness"],
            Self::Lab | Self::Oklab => ["lightness", "a", "b"],
            Self::Lch | Self::Oklch => ["lightness", "chroma", "hue"],
        }
    }
}

/// One entry of `components`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Component {
    Number(f64),
    /// The `"none"` keyword: this component is missing or not applicable, which is not the same as 0.
    None,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Color {
    pub color_space: Node<ValueOrRef<ColorSpace>>,
    /// Either three components that can each be a reference, or one reference to a whole array.
    pub components: Node<ValueOrRef<[Node<ValueOrRef<Component>>; 3]>>,
    /// `None` when omitted, which means fully opaque.
    pub alpha: Option<Node<ValueOrRef<f64>>>,
    /// The fallback in `#rrggbb` form. It is never used to compute the color.
    pub hex: Option<Node<ValueOrRef<String>>>,
}

/// Reads a component from a literal prop: a number or `"none"`.
pub(crate) fn read_component(a: &Analyzed, prop: PropId) -> Option<Component> {
    match a.number(prop) {
        Some(n) => Some(Component::Number(n)),
        None => (a.string(prop) == Some("none")).then_some(Component::None),
    }
}

fn is_hex(s: &str) -> bool {
    s.len() == 7 && s.starts_with('#') && s[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

impl FromProp for Color {
    fn from_prop(prop: PropId, a: &Analyzed, diags: &mut impl DiagnosticSink) -> Option<Self> {
        let Some(members) = a.members(prop) else {
            diags.emit(
                Diagnostic::new(DiagnosticCode::InvalidPropertyShape)
                    .with_message("a color must be an object")
                    .primary(
                        a.span(prop),
                        "expected {\"colorSpace\": \"srgb\", \"components\": [1, 0, 0]}",
                    ),
            );
            return None;
        };

        for (key, &member) in members {
            if !matches!(key.as_str(), "colorSpace" | "components" | "alpha" | "hex") {
                diags.emit(
                    Diagnostic::new(DiagnosticCode::InvalidPropertyShape)
                        .with_message(format!("unknown color property `{key}`"))
                        .primary(
                            a.span(member),
                            "a color only has `colorSpace`, `components`, `alpha` and `hex`",
                        ),
                );
            }
        }

        // Every property is read, so one run reports every problem in the value.
        let color_space = match a.member(prop, "colorSpace") {
            Some(slot) => choice_slot(
                a,
                slot,
                "a color `colorSpace`",
                &ColorSpace::NAMES,
                ColorSpace::parse,
                diags,
            ),
            None => {
                report_missing(a, prop, "colorSpace", diags);
                None
            }
        };
        let space = color_space.as_ref().and_then(|n| match n.value {
            ValueOrRef::Value(s) => Some(s),
            ValueOrRef::Reference(end) => a.string(end).and_then(ColorSpace::parse),
        });

        let components = match a.member(prop, "components") {
            Some(slot) => (a, slot, space, diags),
            None => {
                report_missing(a, prop, "components", diags);
                None
            }
        };
        let alpha = a
            .member(prop, "alpha")
            .map(|slot| alpha_slot(a, slot, diags));
        let hex = a.member(prop, "hex").map(|slot| hex_slot(a, slot, diags));

        let alpha = match alpha {
            Some(alpha) => Some(alpha?),
            None => None,
        };
        let hex = match hex {
            Some(hex) => Some(hex?),
            None => None,
        };
        Some(Color {
            color_space: color_space?,
            components: components?,
            alpha,
            hex,
        })
    }
}

fn report_missing(a: &Analyzed, prop: PropId, key: &str, diags: &mut impl DiagnosticSink) {
    diags.emit(
        Diagnostic::new(DiagnosticCode::InvalidPropertyShape)
            .with_message(format!("a color is missing `{key}`"))
            .primary(a.span(prop), format!("add `{key}` here")),
    );
}

/// Reports a number that the color space does not allow. `at` is where the author wrote it. When it was reached
/// through a reference, `end` is the literal it ended at.
fn check_range(
    a: &Analyzed,
    space: ColorSpace,
    index: usize,
    n: f64,
    at: PropId,
    end: Option<PropId>,
    diags: &mut impl DiagnosticSink,
) -> bool {
    let range = space.ranges()[index];
    if range.contains(n) {
        return true;
    }
    let name = space.component_names()[index];
    let mut diag = Diagnostic::new(DiagnosticCode::InvalidTokenValue)
        .with_message(format!(
            "the {name} of a `{}` color must be {}, but this is {n}",
            space.as_str(),
            range.describe()
        ))
        .primary(
            a.span(at),
            format!("expected a number {}", range.describe()),
        );
    if let Some(end) = end {
        diag = diag.secondary(a.span(end), "the reference ends here");
    }
    diags.emit(diag);
    false
}

fn components_slot(
    a: &Analyzed,
    slot: PropId,
    space: Option<ColorSpace>,
    diags: &mut impl DiagnosticSink,
) -> Option<Node<ValueOrRef<[Node<ValueOrRef<Component>>; 3]>>> {
    match classify_slot(a, slot, diags) {
        SlotKind::Broken => None,
        SlotKind::Reference(target) => {
            let end = checked_reference(
                a,
                slot,
                target,
                "an array of 3 color components",
                |p| {
                    a.items(p).is_some_and(|items| {
                        items.len() == 3 && items.iter().all(|&i| read_component(a, i).is_some())
                    })
                },
                diags,
            )?;
            let mut ok = true;
            if let Some(space) = space {
                for (i, &item) in a.items(end)?.iter().enumerate() {
                    if let Some(Component::Number(n)) = read_component(a, item) {
                        ok &= check_range(a, space, i, n, slot, Some(item), diags);
                    }
                }
            }
            ok.then(|| node(a, slot, ValueOrRef::Reference(end)))
        }
        SlotKind::Literal => {
            let Some(items) = a.items(slot) else {
                diags.emit(
                    Diagnostic::new(DiagnosticCode::InvalidTokenValue)
                        .with_message("color `components` must be an array of 3 entries")
                        .primary(a.span(slot), "expected [1, 0, 0]")
                        .help("each entry is a number or \"none\""),
                );
                return None;
            };
            if items.len() != 3 {
                diags.emit(
                    Diagnostic::new(DiagnosticCode::InvalidTokenValue)
                        .with_message(format!(
                            "color `components` must have exactly 3 entries, found {}",
                            items.len()
                        ))
                        .primary(a.span(slot), "expected 3 entries"),
                );
                return None;
            }

            let parsed: Vec<_> = items
                .iter()
                .enumerate()
                .map(|(i, &item)| component_slot(a, item, space, i, diags))
                .collect();
            let [Some(x), Some(y), Some(z)] = <[_; 3]>::try_from(parsed).ok()? else {
                return None;
            };
            Some(node(a, slot, ValueOrRef::Value([x, y, z])))
        }
    }
}

fn component_slot(
    a: &Analyzed,
    slot: PropId,
    space: Option<ColorSpace>,
    index: usize,
    diags: &mut impl DiagnosticSink,
) -> Option<Node<ValueOrRef<Component>>> {
    match classify_slot(a, slot, diags) {
        SlotKind::Broken => None,
        SlotKind::Reference(target) => {
            let end = checked_reference(
                a,
                slot,
                target,
                "a number or \"none\"",
                |p| read_component(a, p).is_some(),
                diags,
            )?;
            if let (Some(space), Some(Component::Number(n))) = (space, read_component(a, end)) {
                if !check_range(a, space, index, n, slot, Some(end), diags) {
                    return None;
                }
            }
            Some(node(a, slot, ValueOrRef::Reference(end)))
        }
        SlotKind::Literal => match read_component(a, slot) {
            Some(c) => {
                if let (Some(space), Component::Number(n)) = (space, c) {
                    if !check_range(a, space, index, n, slot, None, diags) {
                        return None;
                    }
                }
                Some(node(a, slot, ValueOrRef::Value(c)))
            }
            None => {
                diags.emit(
                    Diagnostic::new(DiagnosticCode::InvalidTokenValue)
                        .with_message("a color component must be a number or \"none\"")
                        .primary(a.span(slot), "expected a number or \"none\"")
                        .help(alias_help(a, slot)),
                );
                None
            }
        },
    }
}

fn alpha_slot(
    a: &Analyzed,
    slot: PropId,
    diags: &mut impl DiagnosticSink,
) -> Option<Node<ValueOrRef<f64>>> {
    let alpha = number_slot(a, slot, "a color `alpha`", diags)?;
    let (n, end) = match alpha.value {
        ValueOrRef::Value(n) => (n, None),
        ValueOrRef::Reference(end) => (a.number(end)?, Some(end)),
    };
    if (0.0..=1.0).contains(&n) {
        return Some(alpha);
    }
    let mut diag = Diagnostic::new(DiagnosticCode::InvalidTokenValue)
        .with_message(format!(
            "a color `alpha` must be between 0 and 1, but this is {n}"
        ))
        .primary(a.span(slot), "expected a number between 0 and 1");
    if let Some(end) = end {
        diag = diag.secondary(a.span(end), "the reference ends here");
    }
    diags.emit(diag);
    None
}

fn hex_slot(
    a: &Analyzed,
    slot: PropId,
    diags: &mut impl DiagnosticSink,
) -> Option<Node<ValueOrRef<String>>> {
    match classify_slot(a, slot, diags) {
        SlotKind::Broken => None,
        SlotKind::Reference(target) => {
            let end = checked_reference(
                a,
                slot,
                target,
                "a 6 digit hex color such as \"#ff00ff\"",
                |p| a.string(p).is_some_and(is_hex),
                diags,
            )?;
            Some(node(a, slot, ValueOrRef::Reference(end)))
        }
        SlotKind::Literal => match a.string(slot) {
            Some(s) if is_hex(s) => Some(node(a, slot, ValueOrRef::Value(s.to_string()))),
            _ => {
                diags.emit(
                    Diagnostic::new(DiagnosticCode::InvalidTokenValue)
                        .with_message("a color `hex` must be 6 digit hex notation")
                        .primary(a.span(slot), "expected a string like \"#ff00ff\"")
                        .help("3, 4 and 8 digit forms are not allowed, because they would conflict with `alpha`"),
                );
                None
            }
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        diagnostics::diagnostic::DiagnosticStore,
        ir::{
            lower::{PropKind, TokenBody},
            merge::LayerOrigin,
            pipeline::{Layer, analyze},
        },
        source::SourceIdGenerator,
    };

    /// Analyzes `{"t": {"$type": "color", "$value": <value>}}` plus `extra` top-level members, and parses the
    /// `$value`.
    fn parse(value: &str, extra: &str) -> (Option<Color>, DiagnosticStore, Analyzed) {
        let doc = format!(r#"{{"t": {{"$type": "color", "$value": {value}}}{extra}}}"#);
        let (tree, errors) = json_spanned::parse_json(&doc);
        assert!(errors.is_empty(), "bad test json: {doc}");
        let tree = tree.unwrap();

        let source = SourceIdGenerator::default().generate();
        let layers = [Layer {
            source,
            origin: LayerOrigin::File(source),
            tree: &tree,
        }];
        let analyzed = analyze(&layers, &mut DiagnosticStore::new()).expect("no cycles");

        let token = analyzed.lowered.member(&["t"]).unwrap();
        let PropKind::Token(data) = &analyzed.lowered.get(token).kind else {
            panic!()
        };
        let TokenBody::Value(v) = data.body else {
            panic!("no $value")
        };

        let mut diags = DiagnosticStore::new();
        let result = Color::from_prop(v, &analyzed, &mut diags);
        (result, diags, analyzed)
    }

    fn codes(d: &DiagnosticStore) -> Vec<DiagnosticCode> {
        d.diagnostics().iter().map(|d| d.code).collect()
    }

    fn ok(value: &str) -> Color {
        let (result, diags, _) = parse(value, "");
        assert!(diags.diagnostics().is_empty(), "{:?}", diags.diagnostics());
        result.expect("a color")
    }

    fn rejected(value: &str) -> Vec<DiagnosticCode> {
        let (result, diags, _) = parse(value, "");
        assert!(result.is_none(), "{value} was accepted");
        codes(&diags)
    }

    #[test]
    fn parses_a_minimal_color() {
        let c = ok(r#"{"colorSpace": "srgb", "components": [1, 0.5, 0]}"#);
        assert_eq!(c.color_space.value, ValueOrRef::Value(ColorSpace::Srgb));
        let ValueOrRef::Value(parts) = &c.components.value else {
            panic!()
        };
        assert_eq!(parts[1].value, ValueOrRef::Value(Component::Number(0.5)));
        assert!(c.alpha.is_none() && c.hex.is_none());
    }

    #[test]
    fn parses_alpha_hex_and_none() {
        let c = ok(
            r##"{"colorSpace": "hsl", "components": ["none", 50, 50], "alpha": 0.5, "hex": "#80FF00"}"##,
        );
        let ValueOrRef::Value(parts) = &c.components.value else {
            panic!()
        };
        assert_eq!(parts[0].value, ValueOrRef::Value(Component::None));
        assert_eq!(c.alpha.unwrap().value, ValueOrRef::Value(0.5));
        assert_eq!(
            c.hex.unwrap().value,
            ValueOrRef::Value("#80FF00".to_string())
        );
    }

    #[test]
    fn every_color_space_name_parses() {
        for name in ColorSpace::NAMES {
            let space = ColorSpace::parse(name).expect(name);
            assert_eq!(space.as_str(), name);
        }
    }

    #[test]
    fn every_space_accepts_a_valid_color() {
        let valid = [
            ("srgb", "[1, 0, 0.5]"),
            ("srgb-linear", "[1, 0, 0.5]"),
            ("display-p3", "[1, 0, 0.5]"),
            ("a98-rgb", "[1, 0, 0.5]"),
            ("prophoto-rgb", "[1, 0, 0.5]"),
            ("rec2020", "[1, 0, 0.5]"),
            ("xyz-d65", "[1, 0, 0.5]"),
            ("xyz-d50", "[1, 0, 0.5]"),
            ("hsl", "[359.9, 100, 0]"),
            ("hwb", "[0, 0, 100]"),
            ("lab", "[100, -160, 160]"),
            ("lch", "[0, 230, 359]"),
            ("oklab", "[1, -0.5, 0.5]"),
            ("oklch", "[0.5, 0.4, 120]"),
        ];
        for (space, components) in valid {
            ok(&format!(
                r#"{{"colorSpace": "{space}", "components": {components}}}"#
            ));
        }
    }

    #[test]
    fn components_are_range_checked_per_color_space() {
        let invalid = [
            ("srgb", "[1.1, 0, 0]"),
            ("srgb", "[0, -0.1, 0]"),
            ("xyz-d65", "[0, 0, 2]"),
            ("hsl", "[360, 50, 50]"),
            ("hsl", "[-1, 50, 50]"),
            ("hsl", "[0, 101, 50]"),
            ("hwb", "[0, 0, 100.5]"),
            ("lab", "[101, 0, 0]"),
            ("lch", "[50, -1, 0]"),
            ("lch", "[50, 10, 360]"),
            ("oklab", "[1.5, 0, 0]"),
            ("oklch", "[0.5, 0.1, 400]"),
        ];
        for (space, components) in invalid {
            let value = format!(r#"{{"colorSpace": "{space}", "components": {components}}}"#);
            assert_eq!(
                rejected(&value),
                [DiagnosticCode::InvalidTokenValue],
                "{value}"
            );
        }
    }

    #[test]
    fn the_same_number_is_fine_in_one_space_and_not_another() {
        ok(r#"{"colorSpace": "hsl", "components": [200, 50, 50]}"#);
        rejected(r#"{"colorSpace": "srgb", "components": [200, 50, 50]}"#);
    }

    #[test]
    fn none_is_allowed_in_every_position() {
        ok(r#"{"colorSpace": "oklch", "components": ["none", "none", "none"]}"#);
    }

    #[test]
    fn components_must_be_exactly_three_numbers_or_none() {
        for components in [
            "[1, 0]",
            "[1, 0, 0, 0]",
            "[]",
            "1",
            r#""red""#,
            r#"{"r": 1}"#,
        ] {
            let value = format!(r#"{{"colorSpace": "srgb", "components": {components}}}"#);
            assert_eq!(
                rejected(&value),
                [DiagnosticCode::InvalidTokenValue],
                "{value}"
            );
        }
        assert_eq!(
            rejected(r#"{"colorSpace": "srgb", "components": [1, "auto", 0]}"#),
            [DiagnosticCode::InvalidTokenValue]
        );
    }

    #[test]
    fn each_bad_component_is_reported() {
        let codes = rejected(r#"{"colorSpace": "srgb", "components": [2, 3, 0]}"#);
        assert_eq!(codes.len(), 2);
    }

    #[test]
    fn alpha_must_be_between_zero_and_one() {
        for alpha in ["1.5", "-0.1", r#""half""#] {
            let value =
                format!(r#"{{"colorSpace": "srgb", "components": [0, 0, 0], "alpha": {alpha}}}"#);
            assert_eq!(
                rejected(&value),
                [DiagnosticCode::InvalidTokenValue],
                "{value}"
            );
        }
        ok(r#"{"colorSpace": "srgb", "components": [0, 0, 0], "alpha": 0}"#);
        ok(r#"{"colorSpace": "srgb", "components": [0, 0, 0], "alpha": 1}"#);
    }

    #[test]
    fn hex_must_be_six_digits() {
        for hex in [
            r##""#fff""##,
            r##""#ff00ff80""##,
            r##""ff00ff""##,
            r##""#gg0000""##,
            "1",
        ] {
            let value =
                format!(r#"{{"colorSpace": "srgb", "components": [1, 0, 0], "hex": {hex}}}"#);
            assert_eq!(
                rejected(&value),
                [DiagnosticCode::InvalidTokenValue],
                "{value}"
            );
        }
    }

    #[test]
    fn an_unknown_color_space_lists_the_choices() {
        let (result, diags, _) = parse(r#"{"colorSpace": "cmyk", "components": [0, 0, 0]}"#, "");
        assert!(result.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidTokenValue]);
        assert!(diags.diagnostics()[0].message.contains("\"oklch\""));
    }

    #[test]
    fn missing_members_are_each_reported() {
        assert_eq!(
            rejected("{}"),
            [
                DiagnosticCode::InvalidPropertyShape,
                DiagnosticCode::InvalidPropertyShape
            ]
        );
    }

    #[test]
    fn unknown_members_are_reported_but_the_value_is_still_read() {
        let (result, diags, _) = parse(
            r#"{"colorSpace": "srgb", "components": [0, 0, 0], "gamut": "p3"}"#,
            "",
        );
        assert!(result.is_some());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidPropertyShape]);
    }

    #[test]
    fn a_value_that_is_not_an_object_is_rejected() {
        assert_eq!(
            rejected(r##""#ff0000""##),
            [DiagnosticCode::InvalidPropertyShape]
        );
    }

    #[test]
    fn every_property_can_be_a_reference() {
        let (result, diags, a) = parse(
            r##"{
                "colorSpace": {"$ref": "#/space/$value"},
                "components": [{"$ref": "#/r/$value"}, 0, "none"],
                "alpha": {"$ref": "#/r/$value"},
                "hex": {"$ref": "#/h/$value"}
            }"##,
            r##", "space": {"$type": "dimension", "$value": "srgb"},
                "r": {"$type": "number", "$value": 0.5},
                "h": {"$type": "color", "$value": "#aabbcc"}"##,
        );
        // The target tokens are not colors, but a reference only needs to end at the right kind of literal.
        assert!(diags.diagnostics().is_empty(), "{:?}", diags.diagnostics());
        let c = result.unwrap();
        assert!(matches!(c.color_space.value, ValueOrRef::Reference(_)));
        let ValueOrRef::Value(parts) = &c.components.value else {
            panic!()
        };
        let ValueOrRef::Reference(end) = parts[0].value else {
            panic!()
        };
        assert_eq!(a.number(end), Some(0.5));
        assert!(matches!(c.alpha.unwrap().value, ValueOrRef::Reference(_)));
        assert!(matches!(c.hex.unwrap().value, ValueOrRef::Reference(_)));
    }

    #[test]
    fn components_can_be_one_reference_to_an_array() {
        let (result, diags, a) = parse(
            r##"{"colorSpace": "srgb", "components": {"$ref": "#/base/$value/components"}}"##,
            r##", "base": {"$type": "color", "$value": {"colorSpace": "srgb", "components": [1, 0, 0]}}"##,
        );
        assert!(diags.diagnostics().is_empty(), "{:?}", diags.diagnostics());
        let ValueOrRef::Reference(end) = result.unwrap().components.value else {
            panic!()
        };
        assert_eq!(a.items(end).unwrap().len(), 3);
    }

    #[test]
    fn a_referenced_array_must_have_three_components() {
        let (result, diags, _) = parse(
            r##"{"colorSpace": "srgb", "components": {"$ref": "#/arr/$value/components"}}"##,
            r##", "arr": {"$type": "color", "$value": {"colorSpace": "srgb", "components": [1, 0]}}"##,
        );
        assert!(result.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidTokenValue]);
    }

    #[test]
    fn a_reference_cannot_smuggle_in_an_out_of_range_value() {
        // A single component, labelled at both ends.
        let (result, diags, _) = parse(
            r##"{"colorSpace": "srgb", "components": [{"$ref": "#/n/$value"}, 0, 0]}"##,
            r##", "n": {"$type": "number", "$value": 7}"##,
        );
        assert!(result.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidTokenValue]);
        assert_eq!(diags.diagnostics()[0].labels.len(), 2);

        // A whole array.
        let (result, diags, _) = parse(
            r##"{"colorSpace": "srgb", "components": {"$ref": "#/arr/$value/components"}}"##,
            r##", "arr": {"$type": "color", "$value": {"colorSpace": "srgb", "components": [0, 2, 0]}}"##,
        );
        assert!(result.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidTokenValue]);

        // Alpha.
        let (result, diags, _) = parse(
            r##"{"colorSpace": "srgb", "components": [0, 0, 0], "alpha": {"$ref": "#/n/$value"}}"##,
            r##", "n": {"$type": "number", "$value": 7}"##,
        );
        assert!(result.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidTokenValue]);
    }

    #[test]
    fn a_referenced_color_space_decides_the_ranges() {
        let (result, diags, _) = parse(
            r##"{"colorSpace": {"$ref": "#/s/$value"}, "components": [200, 50, 50]}"##,
            r##", "s": {"$type": "number", "$value": "hsl"}"##,
        );
        assert!(diags.diagnostics().is_empty(), "{:?}", diags.diagnostics());
        assert!(result.is_some());

        let (result, _, _) = parse(
            r##"{"colorSpace": {"$ref": "#/s/$value"}, "components": [200, 50, 50]}"##,
            r##", "s": {"$type": "number", "$value": "srgb"}"##,
        );
        assert!(result.is_none());
    }

    #[test]
    fn a_ref_to_the_wrong_kind_of_literal_is_rejected() {
        let (result, diags, _) = parse(
            r##"{"colorSpace": {"$ref": "#/n/$value"}, "components": [0, 0, 0]}"##,
            r##", "n": {"$type": "number", "$value": 3}"##,
        );
        assert!(result.is_none());
        assert_eq!(codes(&diags), [DiagnosticCode::InvalidTokenValue]);
    }

    #[test]
    fn curly_aliases_inside_a_value_are_rejected_with_advice() {
        let (result, diags, _) =
            parse(r#"{"colorSpace": "srgb", "components": ["{r}", 0, 0]}"#, "");
        assert!(result.is_none());
        assert!(
            diags.diagnostics()[0]
                .help
                .iter()
                .any(|h| h.contains("$ref"))
        );
    }

    #[test]
    fn an_unresolved_ref_is_left_to_the_reference_pass() {
        let (result, diags, _) = parse(
            r##"{"colorSpace": {"$ref": "#/nope"}, "components": [0, 0, 0]}"##,
            "",
        );
        assert!(result.is_none());
        assert!(diags.diagnostics().is_empty());
    }
}
