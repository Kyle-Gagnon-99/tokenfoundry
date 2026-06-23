//! The `color` module contains the structures and logic for representing resolved color tokens

use crate::{
    ir::token::{ColorTokenValue, color::ColorSpaceString},
    resolved::{ResolutionError, ToResolvedToken, ToResolvedTokenValue, token::TokenNumber},
};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ColorComponentArrayElement {
    Number(TokenNumber),
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ResolvedColorToken {
    pub space: ColorSpaceString,
    pub components: [ColorComponentArrayElement; 3],
    pub alpha: Option<TokenNumber>,
    pub hex: Option<String>,
}

impl ToResolvedTokenValue<ResolvedColorToken> for ColorTokenValue {
    fn to_resolved_token(&self) -> Result<ResolvedColorToken, ResolutionError> {}
}
