//! The `utils` module contains various helper functions and utilities used across the CSS generation process.

use crate::{
    ir::{RefAliasOrLiteral, RefOrLiteral},
    output::css::{
        ast::{CssDeclaration, CssProperty, CssValue},
        context::{CssEmitContext, CssEmitError},
    },
};

pub type CssResult<T> = Result<T, CssEmitError>;
pub type CssValueResult = CssResult<CssValue>;

pub trait ToCssValue {
    fn to_css_value(&self, ctx: &CssEmitContext) -> CssValueResult;
}

pub trait ToCssDeclarations {
    fn to_css_declarations(
        &self,
        base_name: &CssProperty,
        ctx: &CssEmitContext,
    ) -> Result<Vec<CssDeclaration>, CssEmitError>;
}

pub fn single_value_declaration(
    base_name: &CssProperty,
    value: CssValue,
) -> Result<Vec<CssDeclaration>, CssEmitError> {
    Ok(vec![CssDeclaration {
        property: base_name.clone(),
        value,
    }])
}

/// Helper function to resolve a RefAliasOrLiteral into a CssValue, using the provided context for variable references and the literal function for direct values.
///
/// # Arguments
///
/// * `value` - The RefAliasOrLiteral to resolve.
/// * `ctx` - The context used for resolving variable references.
///
/// # Returns
///
/// A Result containing the resolved CssValue or an error if a JSON reference is encountered.
///
/// # Errors
///
/// Returns a CssEmitError::UnexpectedJsonReference if the input is a JSON reference, as JSON references are not expected in this context.
pub fn css_ref_alias_or_literal<T: ToCssValue>(
    value: &RefAliasOrLiteral<T>,
    ctx: &CssEmitContext,
) -> CssValueResult {
    match value {
        RefAliasOrLiteral::Literal(value) => value.to_css_value(ctx),
        RefAliasOrLiteral::Alias(alias) => {
            Ok(CssValue::Var(ctx.var_ref_for_path(&alias.target_path)))
        }
        RefAliasOrLiteral::Ref(json_ref) => Err(CssEmitError::UnexpectedJsonReference {
            pointer: json_ref.reference.pointer.to_string().clone(),
        }),
    }
}

pub fn css_ref_or_literal<T: ToCssValue>(
    value: &RefOrLiteral<T>,
    ctx: &CssEmitContext,
) -> CssValueResult {
    match value {
        RefOrLiteral::Literal(value) => value.to_css_value(ctx),
        RefOrLiteral::Ref(json_ref) => Err(CssEmitError::UnexpectedJsonReference {
            pointer: json_ref.reference.pointer.to_string().clone(),
        }),
    }
}

/// Helper function to extract the inner value from a RefOrLiteral, returning an error if it's a JSON reference.
///
/// # Arguments
///
/// * `value` - The RefOrLiteral to extract the value from.
///
/// # Returns
///
/// A Result containing the extracted value or an error if a JSON reference is encountered.
pub fn extract_value_from_ref_or_literal<T: Clone>(
    value: &RefOrLiteral<T>,
) -> Result<T, CssEmitError> {
    match value {
        RefOrLiteral::Literal(value) => Ok(value.clone()),
        RefOrLiteral::Ref(json_ref) => Err(CssEmitError::UnexpectedJsonReference {
            pointer: json_ref.reference.pointer.to_string().clone(),
        }),
    }
}
