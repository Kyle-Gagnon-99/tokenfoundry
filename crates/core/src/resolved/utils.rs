//! The `utils` module contains utility functions and types for working with resolved tokens

use crate::{ir::RefOrLiteral, resolved::ResolutionError};

pub fn extract_value_from_ref_or_literal<T: Clone>(
    value: &RefOrLiteral<T>,
) -> Result<T, ResolutionError> {
    match value {
        RefOrLiteral::Literal(literal) => Ok(literal.clone()),
        RefOrLiteral::Ref(reference) => Err(ResolutionError::UnresolvedAlias(
            reference.reference.clone().into(),
        )),
    }
}
