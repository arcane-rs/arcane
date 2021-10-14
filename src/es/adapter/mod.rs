//! [`Adapter`] definitions.

pub mod transformer;

#[doc(inline)]
pub use self::transformer::{
    strategy::{self, AnyContext},
    Adapt, Strategy, Transformer,
};

#[doc(inline)]
pub use arcana_core::es::adapter::{
    Adapter, Returning, TransformedStream, Wrapper,
};
