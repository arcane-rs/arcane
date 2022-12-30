//! [`Event`] machinery.

#[doc(inline)]
pub use arcane_core::es::event::{
    Event, Initial, Initialized, Name, Concrete, Revision, Sourced, Sourcing,
};

#[cfg(feature = "derive")]
#[doc(inline)]
pub use arcane_codegen::es::event::{Event, Revised};
#[cfg(feature = "derive")]
#[doc(inline)]
pub use arcane_core::es::event::codegen;
