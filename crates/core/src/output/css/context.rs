//! The `context` module defines the `CssContext`, which holds the options and state needed during the CSS generation process.

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::{
    ir::TokenPath,
    output::css::ast::{CssProperty, CssVarRef},
};

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct CssConfigOptions {
    /// If true, the generated CSS will include comments with token names
    pub include_comments: bool,
    /// The prefix to use for generated CSS variables. For example, if the prefix is "my-design-system", a token named "color-primary" would be generated as `--my-design-system-color-primary`.
    pub variable_prefix: String,
    /// The root selector to use for the generated CSS. For example, if the root selector is ":root", the variables will be defined under `:root { ... }`.
    pub root_selector: String,
    /// If true, the generator will use spaces for indentation in the generated CSS. If false, it will use tabs.
    pub use_spaces_for_indentation: bool,
    /// The settings for how to handle composite tokens (like borders, shadows, gradients, etc.) during generation.
    pub composite_options: CssCompositeOptions,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, JsonSchema)]
pub struct CssCompositeOptions {
    /// If true, the generator will expand border tokens into their individual properties (width, style, color).
    pub expand_border: bool,
    /// If true, the generator will expand shadow tokens into their individual properties (offset-x, offset-y, blur-radius, spread-radius, color).
    pub expand_shadow: bool,
    /// If true, the generator will expand gradient tokens into their individual properties (type, colors, positions).
    pub expand_gradient: bool,
    /// If true, the generator will expand transition tokens into their individual properties (property, duration, timing-function, delay).
    pub expand_transition: bool,
    /// If true, the generator will expand stroke style tokens into their individual properties (width, color, dash array, dash offset).
    pub expand_stroke_style: bool,
}

impl Default for CssConfigOptions {
    fn default() -> Self {
        Self {
            include_comments: false,
            variable_prefix: String::new(),
            root_selector: ":root".to_string(),
            use_spaces_for_indentation: true,
            composite_options: CssCompositeOptions::default(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CssEmitContext {
    pub options: CssConfigOptions,
}

impl CssEmitContext {
    pub fn new(options: CssConfigOptions) -> Self {
        Self { options }
    }

    pub fn var_name_for_path(&self, path: &TokenPath) -> CssProperty {
        // If a variable prefix is set, prepend it to the token path segments to form the CSS variable name
        let var_name = if !self.options.variable_prefix.is_empty() {
            format!(
                "--{}-{}",
                self.options.variable_prefix,
                path.segments.join("-").replace('_', "-")
            )
        } else {
            format!("--{}", path.segments.join("-").replace('_', "-"))
        };

        // Return the variable name as a custom CSS property
        CssProperty::Custom(var_name)
    }

    pub fn var_ref_for_path(&self, path: &TokenPath) -> CssVarRef {
        CssVarRef {
            name: self.var_name_for_path(path).into(),
            fallback: None,
        }
    }
}

#[derive(Debug, Clone, thiserror::Error)]
pub enum CssEmitError {
    #[error("JSON reference encountered during CSS emission, which is not supported: {pointer}")]
    UnexpectedJsonReference { pointer: String },

    #[error("unsupported CSS value at {path}: {reason}")]
    UnsupportedCssValue { path: String, reason: String },

    #[error("invalid color space")]
    InvalidColorSpace,
}
