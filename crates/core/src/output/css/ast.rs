//! The `ast` module defines the abstract syntax tree (AST) structures used for representing CSS output in a structured way.
//! This allows for easier generation of CSS code from the internal representation of design tokens

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Represents the separator used in CSS function arguments or lists of values.
pub enum CssSeparator {
    Space,
    Comma,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Represents a reference to a CSS variable.
pub struct CssVarRef {
    pub name: String,
    pub fallback: Option<Box<CssValue>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CssUnit {
    Px,
    Rem,
    Em,
    Percent,
    Ms,
    S,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Represents a CSS value, which can be a raw string, an identifier, a dimension (like `10px`), a function call (like `rgba(255, 0, 0, 1)`), a variable reference (like `var(--my-token)`), or a list of values (like `10px 20px`).
pub enum CssValue {
    Raw(String),
    Ident(String),
    Dimension {
        value: String,
        unit: String,
    },
    Duration {
        value: String,
        unit: CssUnit,
    },
    Function {
        name: String,
        args: Vec<CssValue>,
        separator: CssSeparator,
    },
    Var(CssVarRef),
    List {
        separator: CssSeparator,
        items: Vec<CssValue>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Represents a CSS property, which can be either a standard property (like `color` or `margin`) or a custom property (CSS variable).
/// Most of the properties we generate will be custom properties (CSS variables), but this enum allows for flexibility in case we need to represent standard properties in the future.
pub enum CssProperty {
    Custom(String),
    Standard(String),
}

impl Into<String> for CssProperty {
    fn into(self) -> String {
        match self {
            CssProperty::Custom(custom) => custom,
            CssProperty::Standard(standard) => standard,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Represents a CSS declaration, which consists of a property and a value.
/// For example, a declaration could represent something like `color: red;` or `--my-token: 10px;`.
pub struct CssDeclaration {
    pub property: CssProperty,
    pub value: CssValue,
}

impl CssDeclaration {
    pub fn custom_property(name: String, value: CssValue) -> Self {
        Self {
            property: CssProperty::Custom(name),
            value,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Represents a CSS rule, which consists of a selector and a list of declarations.
/// For example, a rule could represent something like:
/// ```css
/// .my-class {
///     color: red;
///     margin: 10px;
/// }
/// ```
///
/// In our case, we mostly expect to generate rules for CSS variables, which would look like:
/// ```css
/// :root {
///     --my-token: 10px;
/// }
/// ```
pub struct CssRule {
    pub selector: String,
    pub declarations: Vec<CssDeclaration>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// Represents a node in the CSS AST, which can be either a comment or a rule.
pub enum CssNode {
    Comment(String),
    Rule(CssRule),
}

#[derive(Debug, Clone, PartialEq, Eq)]
/// The root of a CSS AST, representing an entire CSS document.
pub struct CssDocument {
    pub nodes: Vec<CssNode>,
}
