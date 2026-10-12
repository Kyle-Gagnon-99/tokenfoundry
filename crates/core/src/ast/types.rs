//! The `types` module defines common types used throughout the abstract syntax tree (AST) representation of DTCG.

use ordered_float::OrderedFloat;

/// A floating-point number type used throughout the AST, wrapped in `OrderedFloat` for total ordering.
pub type FloatingPoint<T = f64> = OrderedFloat<T>;
