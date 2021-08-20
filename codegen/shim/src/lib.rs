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

use proc_macro::TokenStream;

/// Macro for deriving [`Event`].
///
/// [`Event`]: arcana_core::Event
#[proc_macro_derive(Event, attributes(event))]
pub fn derive_event(input: TokenStream) -> TokenStream {
    arcana_codegen_impl::event::derive(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Macro for deriving [`VersionedEvent`].
///
/// [`VersionedEvent`]: arcana_core::VersionedEvent
#[proc_macro_derive(VersionedEvent, attributes(event))]
pub fn derive_versioned_event(input: TokenStream) -> TokenStream {
    arcana_codegen_impl::event::versioned::derive(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
