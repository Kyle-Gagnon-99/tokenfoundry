//! Lowered output model for emitters.
//!
//! This layer converts analysis-oriented tokens into emit-ready values.

use crate::{
    analysis::canonical::{CanonicalToken, CanonicalValue},
    ir::{IrTokenValue, TokenId, TokenPath},
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EmittableToken {
    pub token_id: TokenId,
    pub path: TokenPath,
    pub value: EmittableValue,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EmittableValue {
    Alias { target_path: TokenPath },
    Literal(IrTokenValue),
}

#[derive(Debug, thiserror::Error)]
pub enum LoweringError {
    #[error("unsupported JSON reference encountered while lowering token at {path}")]
    UnsupportedReference { path: String },
}

impl EmittableToken {
    pub fn from_canonical(token: &CanonicalToken) -> Result<Self, LoweringError> {
        let value = match &token.value {
            CanonicalValue::Alias { target_path } => EmittableValue::Alias {
                target_path: target_path.clone(),
            },
            CanonicalValue::Literal(literal) => EmittableValue::Literal(literal.clone()),
        };

        Ok(Self {
            token_id: token.token_id,
            path: token.path.clone(),
            value,
        })
    }
}

#[cfg(test)]
mod tests {
    use test_log::test;
    use tracing::info;

    use crate::{
        analysis::canonical::{CanonicalToken, CanonicalValue},
        ir::token::token_types::dimension::{DimensionTokenValue, DimensionUnit, DimensionValue},
        ir::{IrTokenType, IrTokenValue, TokenId, TokenPath},
    };

    use super::{EmittableToken, EmittableValue};

    #[test]
    fn lowers_literal_dimension_token_to_generic_emittable_value() {
        let canonical = CanonicalToken {
            token_id: TokenId(42),
            path: TokenPath::from_segments(["spacing", "sm"]),
            token_type: IrTokenType::Dimension,
            value: CanonicalValue::Literal(IrTokenValue::Dimension(DimensionTokenValue {
                value: crate::ir::RefOrLiteral::Literal(DimensionValue(crate::ir::JsonNumber(
                    serde_json::Number::from(16),
                ))),
                unit: crate::ir::RefOrLiteral::Literal(DimensionUnit::Px),
            })),
        };

        let lowered = EmittableToken::from_canonical(&canonical).expect("token should lower");
        info!("lowered token: {:?}", lowered);

        match lowered.value {
            EmittableValue::Literal(literal) => {
                info!("lowered literal payload: {:?}", literal);
                assert!(matches!(literal, IrTokenValue::Dimension(_)));
            }
            other => panic!("expected literal payload, got {:?}", other),
        }
    }
}
