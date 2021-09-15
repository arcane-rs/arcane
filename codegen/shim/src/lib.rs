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
/// Also, provides a blanket [`Sourced`] implementation for every state, which
/// can be sourced from all the enum variants.
///
/// > __WARNING:__ Currently may not work with complex generics using where
/// >              clause because of `const` evaluation limitations. Should be
/// >              lifted once [rust-lang/rust#57775] is resolved.
///
/// # Variant attributes
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
/// [`Event`]: arcana_core::es::Event
/// [`Sourced`]: arcana_core::es::event::Sourced
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

// TODO describe specialization
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

/// Macro for deriving [`Transformer`] on [`Adapter`] to transform derived
/// [`Event`]s enums.
///
/// # Struct attributes
///
/// #### `#[event(transformer(event = <ty>))]`
///
/// [`Event`] to transform.
///
/// #### `#[event(transformer(transformed = <ty>))]`
///
/// [`Transformer::Transformed`][1] type for [`Transformer`] impl.
///
/// #### `#[event(transformer(context = <ty>))]`
///
/// [`Transformer::Context`][2] type for [`Transformer`] impl.
///
/// #### `#[event(transformer(error = <ty>))]`
///
/// [`Transformer::Error`][3] type for [`Transformer`] impl.
///
/// #### `#[event(transformer(max_number_of_variants = <ty>))]` — optional
///
/// Due to current limitations of const evaluation, we have to limit maximum
/// number of variants for transformed [`Event`]. Default value is
/// [`MAX_NUMBER_OF_VARIANTS`][4].
///
/// Realistically you should decrease this value if you want slightly shorter
/// compile time or increase it in case you have exceeded the default limit
/// (although it's recommended to refactor into sub-enums for better
/// readability).
///
/// # Example
///
/// ```rust
/// # #![feature(generic_associated_types)]
/// #
/// # use std::{any::Any, convert::Infallible};
/// #
/// # use arcana::es::{event, Event, adapter::Transformer};
/// # use derive_more::From;
/// #
/// #[derive(event::Versioned)]
/// #[event(name = "event.in", version = 1)]
/// struct InputEvent;
///
/// #[derive(event::Versioned)]
/// #[event(name = "event.out", version = 1)]
/// struct OutputEvent;
///
/// impl From<InputEvent> for OutputEvent {
///     fn from(_: InputEvent) -> Self {
///         OutputEvent
///     }
/// }
///
/// #[derive(Event, From)]
/// enum InputEvents {
///     Input(InputEvent),
/// }
///
/// #[derive(Event, From)]
/// enum OutputEvents {
///     Output(OutputEvent),
/// }
///
/// #[derive(Transformer)]
/// #[event(
///     transformer(
///         event = InputEvents,
///         transformed = OutputEvents,
///         context = dyn Any,
///         error = Infallible,
///     )
/// )]
/// struct Adapter;
/// ```
///
/// > __NOTE__: Single [`Adapter`] can [`Transformer::transform`][0] multiple
/// >           [`Event`]s.
///
/// ```rust
/// # #![feature(generic_associated_types)]
/// #
/// # use std::{any::Any, convert::Infallible};
/// #
/// # use arcana::es::{event, Event, adapter::Transformer};
/// # use derive_more::From;
/// #
/// # #[derive(event::Versioned)]
/// # #[event(name = "event", version = 1)]
/// # struct InputEvent;
/// #
/// # #[derive(event::Versioned)]
/// # #[event(name = "out", version = 1)]
/// # struct OutputEvents;
/// #
/// # #[derive(Event, From)]
/// # enum FirstInputEvents {
/// #     Input(InputEvent),
/// # }
/// #
/// # #[derive(Event, From)]
/// # enum SecondInputEvents {
/// #     Input(InputEvent),
/// # }
/// #
/// #[derive(Transformer)]
/// #[event(
///     transformer(
///         event(FirstInputEvents, SecondInputEvents),
///         transformed = OutputEvents,
///         context = dyn Any,
///         error = Infallible,
///     )
/// )]
/// struct FirstAdapter;
///
/// // equivalent to previous `derive`
/// #[derive(Transformer)]
/// #[event(
///     transformer(
///         event = FirstInputEvents,
///         transformed = OutputEvents,
///         context = dyn Any,
///         error = Infallible,
///     ),
///     transformer(
///         event = SecondInputEvents,
///         transformed = OutputEvents,
///         context = dyn Any,
///         error = Infallible,
///     ),
/// )]
/// struct SecondAdapter;
/// ```
/// [0]: arcana_core::es::adapter::Transformer::transform()
/// [1]: arcana_core::es::adapter::Transformer::Transformed
/// [2]: arcana_core::es::adapter::Transformer::Context
/// [3]: arcana_core::es::adapter::Transformer::Error
/// [4]: arcana_codegen_impl::es::event::transformer::MAX_NUMBER_OF_VARIANTS
/// [`Adapter`]: arcana_core::es::Adapter
/// [`Event`]: trait@arcana_core::es::Event
/// [`Transformer`]: arcana_core::es::adapter::Transformer
#[proc_macro_derive(EventTransformer, attributes(transformer))]
pub fn derive_event_transformer(input: TokenStream) -> TokenStream {
    codegen::es::event::transformer::derive(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
