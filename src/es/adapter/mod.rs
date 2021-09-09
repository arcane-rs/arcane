//! [`Adapter`] definitions.

pub mod transformer;

#[doc(inline)]
pub use self::transformer::{TransformedBy, Transformer};

#[doc(inline)]
pub use arcana_core::es::adapter::{Adapter, TransformedStream};

#[cfg(feature = "derive")]
#[doc(inline)]
pub use arcana_core::es::adapter::codegen;
