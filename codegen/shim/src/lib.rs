#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_cfg))]
#![cfg_attr(
    feature = "doc",
    deny(rustdoc::broken_intra_doc_links, rustdoc::private_intra_doc_links)
)]
#![deny(
    macro_use_extern_crate,
    nonstandard_style,
    rust_2018_idioms,
    trivial_casts,
    trivial_numeric_casts
)]
#![forbid(non_ascii_idents, unsafe_code)]
#![warn(
    clippy::as_conversions,
    clippy::branches_sharing_code,
    clippy::clone_on_ref_ptr,
    clippy::create_dir,
    clippy::dbg_macro,
    clippy::debug_assert_with_mut_call,
    clippy::decimal_literal_representation,
    clippy::else_if_without_else,
    clippy::empty_line_after_outer_attr,
    clippy::equatable_if_let,
    clippy::exit,
    clippy::expect_used,
    clippy::fallible_impl_from,
    clippy::filetype_is_file,
    clippy::float_cmp_const,
    clippy::fn_to_numeric_cast,
    clippy::get_unwrap,
    clippy::if_then_some_else_none,
    clippy::imprecise_flops,
    clippy::let_underscore_must_use,
    clippy::lossy_float_literal,
    clippy::map_err_ignore,
    clippy::mem_forget,
    clippy::missing_const_for_fn,
    clippy::missing_docs_in_private_items,
    clippy::multiple_inherent_impl,
    clippy::mutex_integer,
    clippy::nonstandard_macro_braces,
    clippy::option_if_let_else,
    clippy::panic_in_result_fn,
    clippy::pedantic,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::rc_buffer,
    clippy::rc_mutex,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::same_name_method,
    clippy::shadow_unrelated,
    clippy::str_to_string,
    clippy::string_add,
    clippy::string_lit_as_bytes,
    clippy::string_to_string,
    clippy::suboptimal_flops,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::trivial_regex,
    clippy::unimplemented,
    clippy::unnecessary_self_imports,
    clippy::unneeded_field_pattern,
    clippy::unwrap_in_result,
    clippy::unwrap_used,
    clippy::use_debug,
    clippy::use_self,
    clippy::useless_let_if_seq,
    clippy::verbose_file_reads,
    clippy::wildcard_enum_match_arm,
    future_incompatible,
    meta_variable_misuse,
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    noop_method_call,
    semicolon_in_expressions_from_macros,
    unreachable_pub,
    unused_crate_dependencies,
    unused_extern_crates,
    unused_import_braces,
    unused_labels,
    unused_lifetimes,
    unused_qualifications,
    unused_results,
    variant_size_differences
)]

// Only for doc tests.
#[cfg(test)]
use arcana as _;
// Only for generating documentation.
#[cfg(feature = "doc")]
use arcana_core as _;

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
/// - [`event::Sourced`] for every state, which can be sourced from all enum
///   variants;
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
/// [`Adapter`]: arcana_core::es::event::Adapter
/// [`Event`]: arcana_core::es::Event
/// [`event::Initialized`]: arcana_core::es::event::Initialized
/// [`event::Sourced`]: arcana_core::es::event::Sourced
/// [`Transformer`]: arcana_core::es::event::adapter::Transformer
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

/// Macro for deriving [`adapter::Returning`][0] which is required for
/// [`Adapter`] blanket impl.
///
/// # Attributes
///
/// #### `#[adapter(transformed = <ty>)]`
///
/// Aliases: `#[adapter(into = <ty>)]`
///
/// [`adapter::Returning::Transformed`][1] associated type.
///
/// #### `#[adapter(error = <ty>)]` (optional)
///
/// Aliases: `#[adapter(err = <ty>)]`
///
/// [`adapter::Returning::Error`][2] associated type. [`Infallible`] by default.
///
/// # Example
///
/// ```rust
/// # use arcana::es::event;
/// #
/// # #[derive(event::Versioned)]
/// # #[event(name = "event", version = 1)]
/// # struct Event;
/// #
/// #[derive(event::Adapter, Clone, Copy, Debug)]
/// #[adapter(into = Event)]
/// struct Adapter;
/// ```
///
/// [`Adapter`]: arcana_core::es::event::Adapter
/// [`Infallible`]: std::convert::Infallible
/// [0]: arcana_core::es::event::adapter::Returning
/// [1]: arcana_core::es::event::adapter::Returning::Transformed
/// [2]: arcana_core::es::event::adapter::Returning::Error
#[proc_macro_derive(EventAdapter, attributes(adapter))]
pub fn derive_event_adapter(input: TokenStream) -> TokenStream {
    codegen::es::event::adapter::derive(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
