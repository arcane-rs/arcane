#![doc = include_str!("../README.md")]
#![deny(
    nonstandard_style,
    rust_2018_idioms,
    rustdoc::broken_intra_doc_links,
    rustdoc::private_intra_doc_links,
    trivial_casts,
    trivial_numeric_casts
)]
#![forbid(non_ascii_idents, unsafe_code)]
#![warn(
    deprecated_in_future,
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    unreachable_pub,
    unused_import_braces,
    unused_labels,
    unused_qualifications,
    unused_results
)]

#[doc(inline)]
pub use arcana_core::{Event, VersionedEvent};

/// Macro for deriving [`VersionedEvent`](trait@VersionedEvent).
pub use arcana_codegen::VersionedEvent;
