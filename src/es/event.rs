//! [`Event`] machinery.

#[doc(inline)]
pub use arcane_core::es::event::{
    reflect, Concrete, Event, Initial, Initialized, Name, Revisable, Revision,
    RevisionOf, Sourced, Sourcing, Static, Version,
};

#[cfg(feature = "derive")]
#[doc(inline)]
pub use arcane_codegen::es::event::Event;
#[cfg(feature = "derive")]
#[doc(hidden)]
pub mod codegen {
    #[doc(inline)]
    pub use arcane_core::es::event::codegen::*;

    #[doc(inline)]
    pub use arcane_core::const_concat_slices;
}
