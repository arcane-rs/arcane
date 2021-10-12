#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(
    feature = "doc",
    deny(rustdoc::broken_intra_doc_links, rustdoc::private_intra_doc_links)
)]
#![deny(
    nonstandard_style,
    rust_2018_idioms,
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

use arcana_codegen_impl as codegen;
use proc_macro::TokenStream;

/// Macro for deriving [`Event`] on enums.
///
/// For structs consider using [`#[derive(Versioned)]`](macro@VersionedEvent).
///
/// This macro ensures that every combination of [`Event::name`][0] and
/// [`Event::version`][1] corresponds to a single Rust type. The only limitation
/// is that all the underlying [`Event`] or [`Versioned`] impls should be
/// derived too.
///
/// Also, provides a blanket [`event::Sourced`] implementation for every state,
/// which can be sourced from all the enum variants.
///
/// > __WARNING:__ Currently may not work with complex generics using where
/// >              clause because of `const` evaluation limitations. Should be
/// >              lifted once [rust-lang/rust#57775] is resolved.
///
/// # Blanket implementations
///
/// - [`Sourced`] for every state, which can be sourced from all enum variants;
/// - [`Transformer`] for every [`Adapter`], that can transform all enum
///   variants.
///
/// # Variant attributes
///
/// #### `#[event(init)]` (optional)
///
/// Aliases: `#[event(initial)]`
///
/// Use this on a particular enum variant to specify that it should be
/// [`event::Initialized`] rather than [`event::Sourced`].
///
/// #### `#[event(ignore)]` (optional)
///
/// Aliases: `#[event(skip)]`
///
/// Use this on a particular enum variant to completely ignore it in code
/// generation.
///
/// > __WARNING:__ Calling [`Event::name()`][0] or [`Event::version()`][1] on
/// >              ignored variants will result in [`unreachable!`] panic.
///
/// # Example
///
/// ```rust,compile_fail,E0080
/// # #![feature(generic_associated_types)]
/// #
/// # use arcana::es::{event, Event};
/// #
/// #[derive(event::Versioned)]
/// #[event(name = "chat", version = 1)]
/// struct ChatEvent;
///
/// #[derive(event::Versioned)]
/// #[event(name = "chat", version = 1)]
/// struct DuplicateChatEvent;
///
/// // This fails to compile as contains different Rust types with the same
/// // `event::Name` and `event::Version`.
/// #[derive(Event)]
/// enum AnyEvent {
///     Chat(ChatEvent),
///     DuplicateChat(DuplicateChatEvent),
/// }
/// ```
///
/// ```rust
/// # #![feature(generic_associated_types)]
/// #
/// # use arcana::es::{event, Event};
/// #
/// # #[derive(event::Versioned)]
/// # #[event(name = "chat", version = 1)]
/// # struct ChatEvent;
/// #
/// # #[derive(event::Versioned)]
/// # #[event(name = "chat", version = 1)]
/// # struct DuplicateChatEvent;
/// #
/// #[derive(Event)]
/// enum AnyEvent {
///     Chat(ChatEvent),
///     #[event(ignore)] // not recommended for real usage
///     DuplicateChat(DuplicateChatEvent),
/// }
///
/// // This example doesn't need `#[event(ignore)]` attribute, as each
/// // combination of `event::Name` and `event::Version` corresponds to a single
/// // Rust type.
/// #[derive(Event)]
/// enum MoreEvents {
///     Chat(ChatEvent),
///     ChatOnceAgain(ChatEvent),
/// }
/// ```
///
/// [`Adapter`]: arcana_core::es::Adapter
/// [`Event`]: arcana_core::es::Event
/// [`event::Initialized`]: arcana_core::es::event::Initialized
/// [`event::Sourced`]: arcana_core::es::event::Sourced
/// [`Transformer`]: arcana_core::es::adapter::Transformer
/// [`Versioned`]: arcana_core::es::event::Versioned
/// [0]: arcana_core::es::Event::name()
/// [1]: arcana_core::es::Event::version()
/// [rust-lang/rust#57775]: https://github.com/rust-lang/rust/issues/57775
#[proc_macro_derive(Event, attributes(event))]
pub fn derive_event(input: TokenStream) -> TokenStream {
    codegen::es::event::derive(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}

/// Macro for deriving [`Versioned`] on structs.
///
/// For enums consisting of different [`Versioned`] events consider using
/// [`#[derive(Event)]`](macro@Event).
///
/// # Struct attributes
///
/// #### `#[event(name = "...")]`
///
/// Value of [`Versioned::NAME`][0] constant.
///
/// #### `#[event(version = <non-zero-u16>)]`
///
/// Aliases: `#[event(ver = <non-zero-u16>)]`
///
/// Value of [`Versioned::VERSION`][1] constant.
///
/// # Example
///
/// ```rust
/// # use arcana::es::event;
/// #
/// #[derive(event::Versioned)]
/// #[event(name = "event", version = 1)]
/// struct Event;
/// ```
///
/// [`Versioned`]: arcana_core::es::event::Versioned
/// [0]: arcana_core::es::event::Versioned::NAME
/// [1]: arcana_core::es::event::Versioned::VERSION
#[proc_macro_derive(VersionedEvent, attributes(event))]
pub fn derive_versioned_event(input: TokenStream) -> TokenStream {
    codegen::es::event::versioned::derive(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
