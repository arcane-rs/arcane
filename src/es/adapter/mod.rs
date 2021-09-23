//! [`Adapter`] definitions.

pub mod transformer;

#[doc(inline)]
pub use self::transformer::Transformer;

#[doc(inline)]
pub use arcana_core::{
    and,
    es::adapter::{
        transformer::strategy::{And, Any},
        Adapter, Correct, WithError, Wrapper,
    },
};

#[cfg(feature = "derive")]
#[doc(inline)]
pub use arcana_core::es::adapter::codegen;
