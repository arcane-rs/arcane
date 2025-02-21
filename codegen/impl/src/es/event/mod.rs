//! `#[derive(Event)]` macro implementation.

pub mod impl_enum;
pub mod impl_struct;

use proc_macro2::TokenStream;
use syn::spanned::Spanned as _;
use synthez::ToTokens as _;

/// Expands `#[derive(Event)]` macro.
///
/// # Errors
///
/// - If `input` isn't a Rust enum/struct definition;
/// - If failed to parse [`impl_enum::Definition`]
///   or [`impl_struct::Definition`].
pub fn derive(input: TokenStream) -> syn::Result<TokenStream> {
    let input = syn::parse2::<syn::DeriveInput>(input)?;
    Ok(match &input.data {
        syn::Data::Struct(_) => {
            impl_struct::Definition::try_from(input)?.into_token_stream()
        }
        syn::Data::Enum(_) => {
            impl_enum::Definition::try_from(input)?.into_token_stream()
        }
        syn::Data::Union(_) => {
            return Err(syn::Error::new(
                input.span(),
                "union types are not supported",
            ));
        }
    })
}
