//! [`Event`] machinery.

#[doc(inline)]
pub use arcane_core::es::event::{
    Event, Initial, Initialized, Name, Revised, Revision, Sourced, Sourcing,
};

#[cfg(feature = "derive")]
#[doc(inline)]
pub use arcane_codegen::es::event::{Event, Revised};
#[cfg(feature = "derive")]
#[doc(inline)]
pub use arcane_core::es::event::codegen;
