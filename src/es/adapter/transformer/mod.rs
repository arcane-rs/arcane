//! [`Transformer`] definitions.

pub mod strategy;

#[doc(inline)]
pub use self::strategy::Strategy;

#[doc(inline)]
pub use arcana_core::es::adapter::transformer::{
    specialization, TransformedBy, Transformer, WithStrategy,
};

#[cfg(feature = "derive")]
#[doc(inline)]
pub use arcana_codegen::es::transformer::Transformer;
#[cfg(feature = "derive")]
#[doc(hidden)]
// TODO: Replace with panic once `const_panic` is stabilized.
//       https://github.com/rust-lang/rust/issues/51999
pub use arcana_codegen::sa::const_assert as wrong_number_of_events;
