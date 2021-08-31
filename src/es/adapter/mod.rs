//! [`Adapter`] definitions.

pub mod transformer;

#[doc(inline)]
pub use arcana_core::es::adapter::{Adapter, TransformedStream, Transformer};

pub use arcana_codegen::es::adapter::EventTransformer as Transformer;
