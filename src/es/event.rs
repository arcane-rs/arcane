//! [`Event`] machinery.

#[doc(inline)]
pub use arcane_core::es::event::{
    Concrete, Event, Initial, Initialized, Name, Revisable, Revision, Sourced,
    Sourcing, Static, Version,
};

#[cfg(feature = "derive")]
#[doc(inline)]
pub use arcane_codegen::es::event::Event;
#[cfg(feature = "derive")]
#[doc(inline)]
pub use arcane_core::es::event::codegen;
