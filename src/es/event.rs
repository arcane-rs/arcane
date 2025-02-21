//! [`Event`] machinery.

#[cfg(feature = "derive")]
#[doc(hidden)]
pub mod codegen {
    #[doc(inline)]
    pub use arcane_codegen::const_concat_slices;
    #[doc(inline)]
    pub use arcane_codegen::es::event::{
        Reflect, concat_slices, has_different_types_with_same_name_and_revision,
    };
}

#[cfg(feature = "derive")]
#[doc(inline)]
pub use arcane_codegen::es::event::Event;
#[cfg(feature = "reflect")]
#[doc(inline)]
pub use arcane_core::es::event::reflect;
#[doc(inline)]
pub use arcane_core::es::event::{
    Concrete, Event, Initial, Initialized, Name, Revisable, Revision,
    RevisionOf, Sourced, Sourcing, Static, Version,
};
