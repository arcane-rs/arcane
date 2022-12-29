//! [`Event`] machinery.

#[doc(inline)]
pub use arcane_core::es::event::{
    Event, Initial, Initialized, Name, Revised, Revision, Sourced, Sourcing, Meta,
    reflect,
};

#[cfg(feature = "derive")]
#[doc(inline)]
pub use arcane_codegen::es::event::{Event, Revised};
#[cfg(feature = "derive")]
#[doc(hidden)]
pub mod codegen {
    //! Not a public API.

    #[doc(inline)]
    pub use arcane_core::es::event::codegen::*;

    #[doc(inline)]
    pub use arcane_core::const_concat_slices;
}
