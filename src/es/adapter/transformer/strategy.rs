//! [`Strategy`] definition and default implementations.

#[doc(inline)]
pub use arcana_core::es::adapter::transformer::strategy::{
    AsIs, Custom, CustomTransformer, Initialized, Into, Skip, Split, Splitter,
    Strategy,
};

#[cfg(feature = "derive")]
#[doc(inline)]
pub use arcana_codegen::es::strategy::Strategy;
