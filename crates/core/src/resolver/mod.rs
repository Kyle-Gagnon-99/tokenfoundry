//! The resolver turns a DTCG resolver document (sets, modifiers and `resolutionOrder`, see
//! <https://www.designtokens.org/tr/2025.10/resolver/>) plus an [`Input`] into the ordered token layers that the
//! compiler pipeline merges.
//!
//! This crate never opens files. Hosts hand over contents through [`SourceProvider`] or [`MemorySources`].
//!
//! ```text
//! resolver text ─► document (validate, expand refs) ─► Bundle::load (files via provider) ─► layers(input) ─► compile
//! ```

pub mod bundle;
pub mod document;
pub mod input;
pub mod provider;

pub use bundle::Bundle;
pub use document::{Modifier, OrderItem, ResolverDocument, Set, TokenSource};
pub use input::{Input, Selection, all_inputs};
pub use provider::{MemorySources, ProviderError, SourceProvider};
