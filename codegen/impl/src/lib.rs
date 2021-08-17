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

mod event;

use proc_macro::TokenStream;

/// Macro for deriving `arcana::Event`.
///
/// # Attribute arguments
///
/// - `#[event(skip(unique_event_type_and_ver))]` — optional
///
///   Use this value on whole container or particular enum variant to skip check
///   for unique combination of `event_type` and `ver`.
#[proc_macro_derive(Event, attributes(event))]
pub fn derive_event(input: TokenStream) -> TokenStream {
    event::derive(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Macro for deriving `arcana::VersionedEvent`.
///
/// # Attribute arguments
///
/// - `#[event(type = "...")]` — required
///
///   Value used in `fn event_type()` impl.
///
/// - `#[event(ver = u16)]` — required
///
///   Value used in `fn ver()` impl.
#[proc_macro_derive(VersionedEvent, attributes(event))]
pub fn derive_versioned_event(input: TokenStream) -> TokenStream {
    event::versioned::derive(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
