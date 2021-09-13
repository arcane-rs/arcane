//! [`Event`] machinery.

#[doc(inline)]
pub use arcana_core::es::event::{
    Event, Initial, Initialized, Name, Sourced, Sourcing, Upcast, Version,
    Versioned,
};

#[cfg(feature = "derive")]
#[doc(inline)]
pub use arcana_codegen::es::event::{Event, Versioned};
#[cfg(feature = "derive")]
#[doc(hidden)]
// Named so long for better error messages
// TODO: Replace with panic once `const_panic` is stabilized.
//       https://github.com/rust-lang/rust/issues/51999
#[rustfmt::skip]
pub use arcana_codegen::sa::const_assert as
    each_combination_of_name_and_version_must_correspond_to_single_type;
#[cfg(feature = "derive")]
#[doc(inline)]
pub use arcana_core::es::event::codegen;
