//! [`Event`] machinery.

#[cfg(feature = "derive")]
#[doc(inline)]
pub use arcana_codegen::es::event::{Event, Versioned};

#[doc(inline)]
pub use arcana_core::es::event::{
    Event, Initial, Initialized, Name, Sourced, Version, Versioned,
};

#[doc(hidden)]
pub use arcana_core::es::event::{BorrowInitial, UnpackInitial};
