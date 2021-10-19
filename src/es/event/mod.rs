//! [`Event`] machinery.

pub mod adapter;

#[doc(inline)]
pub use arcana_core::es::event::{
    Adapter, Event, Initial, Initialized, Name, Raw, Sourced, Sourcing,
    Version, Versioned, VersionedOrRaw,
};

#[cfg(feature = "derive")]
#[doc(inline)]
pub use arcana_codegen::es::event::{Adapter, Event, Versioned};
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
