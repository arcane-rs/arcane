// NOTICE: Unfortunately this macro MUST be defined here, in the crate's root module, because Rust
//         doesn't allow to export `macro_rules!` macros from a `proc-macro` crate type currently,
//         and so we cannot move the definition into a sub-module and use the `#[macro_export]`
//         attribute.
/// Attempts to merge an [`Option`]ed `$field` of a `$self` struct with the same
/// `$field` of `$another` struct. If both are [`Some`], then throws a
/// "duplicated argument" error with a [`Span`] related to the `$another` struct
/// (a later one).
///
/// The type of [`Span`] may be explicitly specified as one of the
/// [`SpanContainer`] methods.
/// By default, [`SpanContainer::span_ident`] is used.
///
/// [`Span`]: proc_macro2::Span
/// [`SpanContainer`]: crate::util::span_container::SpanContainer
/// [`SpanContainer::span_ident`]: crate::util::span_container::SpanContainer::span_ident
macro_rules! try_merge_opt {
    ($field:ident: $self:ident, $another:ident => $span:ident) => {{
        if let Some(v) = $self.$field {
            $another
                .$field
                .replace(v)
                .none_or_else(|dup| crate::common::parse::attr::err::dup_arg(&dup.$span()))?;
        }
        $another.$field
    }};

    ($field:ident: $self:ident, $another:ident) => {
        try_merge_opt!($field: $self, $another => span_ident)
    };
}

pub mod common;
#[cfg(feature = "es")]
pub mod es;

use proc_macro_error::abort_call_site;
use proc_macro2::TokenStream;

/// Performs expansion of the given `proc_macro_derive` `implementation`.
///
/// # Panics
///
/// Any happened error is reported via [`proc_macro_error`] as a panic, so use
/// this attribute to catch them.
pub fn expand_derive<I, O>(
    input: I,
    implementation: fn(syn::DeriveInput) -> TokenStream,
) -> O
where
    TokenStream: From<I>,
    O: From<TokenStream>,
{
    syn::parse2(input.into())
        .map(implementation)
        .unwrap_or_else(|err| abort_call_site!(err))
        .into()
}
