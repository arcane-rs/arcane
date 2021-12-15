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
#[doc(inline)]
pub use arcana_core::es::event::codegen;
