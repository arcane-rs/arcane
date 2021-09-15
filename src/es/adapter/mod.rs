//! [`Adapter`] definitions.

pub mod transformer;

#[doc(inline)]
pub use self::transformer::Transformer;

#[doc(inline)]
pub use arcana_core::es::adapter::{
    Adapter, TransformedStream, WithError, Wrapper,
};

#[cfg(feature = "derive")]
#[doc(inline)]
pub use arcana_core::es::adapter::codegen;
