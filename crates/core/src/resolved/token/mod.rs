//! The `token` module contains the structures for representing resolved tokens that come from the IR and are ready to be transformed into various output formats.

use ordered_float::OrderedFloat;
use serde_json::Number;

pub mod color;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct TokenNumber(pub OrderedFloat<f64>);

impl From<f64> for TokenNumber {
    fn from(value: f64) -> Self {
        TokenNumber(OrderedFloat(value))
    }
}

impl From<Number> for TokenNumber {
    fn from(value: Number) -> Self {
        TokenNumber(OrderedFloat(value.as_f64().unwrap()))
    }
}
