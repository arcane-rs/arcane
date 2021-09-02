#![doc(hidden)]

#[doc(hidden)]
#[rustfmt::skip]
pub use arcana_codegen::{
    // Named so long for better error messages
    // TODO: replace with panic once const_panic is stabilized
    //       https://github.com/rust-lang/rust/issues/51999
    sa::const_assert as
    every_combination_of_event_name_and_version_must_correspond_to_single_type,
    unique_events::{self, UniqueEvents},
};
