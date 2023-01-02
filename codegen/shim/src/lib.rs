#![doc = include_str!("../README.md")]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]
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
    clippy::as_ptr_cast_mut,
    clippy::assertions_on_result_states,
    clippy::branches_sharing_code,
    clippy::clone_on_ref_ptr,
    clippy::create_dir,
    clippy::dbg_macro,
    clippy::debug_assert_with_mut_call,
    clippy::decimal_literal_representation,
    clippy::default_union_representation,
    clippy::derive_partial_eq_without_eq,
    clippy::else_if_without_else,
    clippy::empty_drop,
    clippy::empty_line_after_outer_attr,
    clippy::empty_structs_with_brackets,
    clippy::equatable_if_let,
    clippy::exit,
    clippy::expect_used,
    clippy::fallible_impl_from,
    clippy::filetype_is_file,
    clippy::float_cmp_const,
    clippy::fn_to_numeric_cast,
    clippy::fn_to_numeric_cast_any,
    clippy::format_push_string,
    clippy::get_unwrap,
    clippy::if_then_some_else_none,
    clippy::imprecise_flops,
    clippy::index_refutable_slice,
    clippy::iter_on_empty_collections,
    clippy::iter_on_single_items,
    clippy::iter_with_drain,
    clippy::large_include_file,
    clippy::lossy_float_literal,
    clippy::map_err_ignore,
    clippy::mem_forget,
    clippy::missing_const_for_fn,
    clippy::missing_docs_in_private_items,
    clippy::multiple_inherent_impl,
    clippy::mutex_atomic,
    clippy::mutex_integer,
    clippy::nonstandard_macro_braces,
    clippy::option_if_let_else,
    clippy::panic_in_result_fn,
    clippy::partial_pub_fields,
    clippy::pedantic,
    clippy::print_stderr,
    clippy::print_stdout,
    clippy::rc_buffer,
    clippy::rc_mutex,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::same_name_method,
    clippy::shadow_unrelated,
    clippy::significant_drop_in_scrutinee,
    clippy::str_to_string,
    clippy::string_add,
    clippy::string_lit_as_bytes,
    clippy::string_slice,
    clippy::string_to_string,
    clippy::suboptimal_flops,
    clippy::suspicious_operation_groupings,
    clippy::todo,
    clippy::trailing_empty_array,
    clippy::transmute_undefined_repr,
    clippy::trivial_regex,
    clippy::try_err,
    clippy::undocumented_unsafe_blocks,
    clippy::unimplemented,
    clippy::unnecessary_self_imports,
    clippy::unneeded_field_pattern,
    clippy::unused_peekable,
    clippy::unwrap_in_result,
    clippy::unwrap_used,
    clippy::use_debug,
    clippy::use_self,
    clippy::useless_let_if_seq,
    clippy::verbose_file_reads,
    clippy::wildcard_enum_match_arm,
    future_incompatible,
    let_underscore_drop,
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
    unused_tuple_struct_fields,
    variant_size_differences
)]

// Only for doc tests.
#[cfg(test)]
use arcane as _;
// Only for generating documentation.
#[cfg(feature = "doc")]
use arcane_core as _;

use arcane_codegen_impl as codegen;
use proc_macro::TokenStream;

/// Macro for deriving [`Event`] on structs and enums.
///
/// # Enums
///
/// This macro provides [`Event`] (and [`event::Revisable`] optionally)
/// implementations for the enum. The enum must have a single-single variants,
/// implementing [`Event`] (and [`event::Revisable`] optionally).
///
/// The macro ensures that every combination of [`Event::name`][0]
/// (and [`event::Revisable::revision`][1] optionally) corresponds to a single
/// Rust type. The only limitation is that all the underlying [`Event`]
/// (and [`event::Revisable`] optionally) impls should be derived too.
///
/// Also, provides a blanket [`event::Sourced`] implementation for every state,
/// which can be sourced from all the enum variants.
///
/// > **WARNING:** Currently may not work with complex generics using where
/// >              clause because of `const` evaluation limitations. Should be
/// >              lifted once [rust-lang/rust#57775] is resolved.
///
/// ## Variant attributes
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
/// > **WARNING:** Calling [`Event::name()`][0]
///                or [`event::Revisalbe::revision()`][1] on ignored variants
///                will result in [`unreachable!`] panic.
///
/// ## Example
///
/// ```rust,compile_fail,E0080
/// # use arcane::es::Event;
/// #
/// #[derive(Event)]
/// #[event(name = "chat", revision = 1)]
/// struct ChatEvent;
///
/// #[derive(Event)]
/// #[event(name = "chat", revision = 1)]
/// struct DuplicateChatEvent;
///
/// // This fails to compile as contains different Rust types with the same
/// // `event::Name` and `event::Revision`.
/// #[derive(Event)]
/// enum AnyEvent {
///     Chat(ChatEvent),
///     DuplicateChat(DuplicateChatEvent),
/// }
/// ```
///
/// ```rust
/// # use arcane::es::Event;
/// #
/// # #[derive(Event)]
/// # #[event(name = "chat", revision = 1)]
/// # struct ChatEvent;
/// #
/// # #[derive(Event)]
/// # #[event(name = "chat", revision = 1)]
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
/// // combination of `event::Name` and `event::Revision` corresponds to
/// // a single Rust type.
/// #[derive(Event)]
/// enum MoreEvents {
///     Chat(ChatEvent),
///     ChatOnceAgain(ChatEvent),
/// }
/// ```
///
/// # Structs
///
/// This macro provides a [`event::Static`] (and [`event::Concrete`] optionally)
/// implementation for the struct.
///
/// ## Struct attributes
///
/// #### `#[event(name = "...")]`
///
/// Value of [`event::Static::NAME`][2] constant.
///
/// #### `#[event(revision = <non-zero-u16>)]` (optional)
///
/// Aliases: `#[event(rev = <non-zero-u16>)]`
///
/// Value of [`event::Concrete::REVISION`][3] constant.
///
/// ## Example
///
/// ```rust
/// # use arcane::es::Event;
/// #
/// #[derive(Event)]
/// #[event(name = "created", revision = 1)]
/// struct Created;
/// ```
///
/// [`Event`]: arcane_core::es::Event
/// [`event::Concrete`]: arcane_core::es::event::Concrete
/// [`event::Initialized`]: arcane_core::es::event::Initialized
/// [`event::Revisable`]: arcane_core::es::event::Revisable
/// [`event::Sourced`]: arcane_core::es::event::Sourced
/// [`event::Static`]: arcane_core::es::event::Static
/// [0]: arcane_core::es::Event::name()
/// [1]: arcane_core::es::event::Revisable::revision()
/// [2]: arcane_core::es::event::Static::NAME
/// [3]: arcane_core::es::event::Concrete::REVISION
/// [rust-lang/rust#57775]: https://github.com/rust-lang/rust/issues/57775
#[proc_macro_derive(Event, attributes(event))]
pub fn derive_event(input: TokenStream) -> TokenStream {
    codegen::es::event::derive(input.into())
        .unwrap_or_else(syn::Error::into_compile_error)
        .into()
}
