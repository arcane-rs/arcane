//! [`Adapter`] definitions.

pub mod transformer;

#[doc(inline)]
pub use self::transformer::{strategy, Adapt, Strategy, Transformer};

#[doc(inline)]
pub use arcana_core::es::event::adapter::{
    Adapter, Returning, TransformedStream, Wrapper,
};

#[cfg(feature = "derive")]
#[doc(inline)]
pub use arcana_codegen::es::event::Adapter;
