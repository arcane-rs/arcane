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
#[proc_macro_derive(Event, attributes(event))]
pub fn derive_event(input: TokenStream) -> TokenStream {
    event::derive(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Macro for deriving `arcana::VersionedEvent`.
#[proc_macro_derive(VersionedEvent, attributes(event))]
pub fn derive_versioned_event(input: TokenStream) -> TokenStream {
    event::versioned::derive(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
