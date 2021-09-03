//! [`Transformer`] definitions.

pub mod strategy;

#[doc(inline)]
pub use self::strategy::Strategy;

#[doc(inline)]
pub use arcana_core::es::adapter::transformer::{Transformer, WithStrategy};

#[cfg(feature = "derive")]
#[doc(inline)]
pub use arcana_codegen::es::transformer::Transformer;