//! `#[derive(event::Adapter)]` macro implementation.

use std::convert::TryFrom;

use proc_macro2::TokenStream;
use quote::quote;
use syn::parse_quote;
use synthez::{ParseAttrs, Required, ToTokens};

/// Expands `#[derive(event::Adapter)]` macro.
///
/// # Errors
///
/// If failed to parse [`Attrs`].
pub fn derive(input: TokenStream) -> syn::Result<TokenStream> {
    let input = syn::parse2::<syn::DeriveInput>(input)?;
    let definition = Definition::try_from(input)?;

    Ok(quote! { #definition })
}

/// Helper attributes of `#[derive(event::Adapter)]` macro.
#[derive(Debug, Default, ParseAttrs)]
pub struct Attrs {
    /// [`Returning::Transformed`][1] associated type.
    ///
    /// [1]: arcana_core::es::event::adapter::Returning::Transformed
    #[parse(value, alias = into)]
    pub transformed: Required<syn::Type>,

    /// [`Returning::Error`][1] associated type.
    ///
    /// [1]: arcana_core::es::event::adapter::Returning::Error
    #[parse(value, alias = err)]
    pub error: Option<syn::Type>,
}

/// Representation of a struct implementing [`event::Adapter`][0], used for
/// code generation.
///
/// [0]: arcana_core::es::event::Adapter
#[derive(Debug, ToTokens)]
#[to_tokens(append(impl_returning))]
pub struct Definition {
    /// [`syn::Ident`](struct@syn::Ident) of this type.
    pub ident: syn::Ident,

    /// [`syn::Generics`] of this type.
    pub generics: syn::Generics,

    /// [`Returning::Transformed`][1] associated type.
    ///
    /// [1]: arcana_core::es::event::adapter::Returning::Transformed
    pub transformed: syn::Type,

    /// [`Returning::Error`][1] associated type.
    ///
    /// [1]: arcana_core::es::event::adapter::Returning::Error
    pub error: syn::Type,
}

impl TryFrom<syn::DeriveInput> for Definition {
    type Error = syn::Error;

    fn try_from(input: syn::DeriveInput) -> syn::Result<Self> {
        let attrs: Attrs = Attrs::parse_attrs("adapter", &input)?;

        Ok(Self {
            ident: input.ident,
            generics: input.generics,
            transformed: attrs.transformed.into_inner(),
            error: attrs
                .error
                .unwrap_or_else(|| parse_quote!(::std::convert::Infallible)),
        })
    }
}

impl Definition {
    /// Generates code to derive [`Returning`][1] trait.
    ///
    /// [1]: arcana_core::es::event::adapter::Returning
    #[must_use]
    pub fn impl_returning(&self) -> TokenStream {
        let ty = &self.ident;
        let (impl_gens, ty_gens, where_clause) = self.generics.split_for_impl();
        let (transformed, error) = (&self.transformed, &self.error);

        quote! {
            #[automatically_derived]
            impl #impl_gens ::arcana::es::event::adapter::Returning for
                 #ty#ty_gens
                 #where_clause
            {
                type Error = #error;
                type Transformed = #transformed;
            }
        }
    }
}

#[cfg(test)]
mod spec {
    use quote::quote;
    use syn::parse_quote;

    #[test]
    fn derives_impl() {
        let input = parse_quote! {
            #[adapter(into = Event, error = CustomError)]
            struct Adapter;
        };

        let output = quote! {
            #[automatically_derived]
            impl ::arcana::es::event::adapter::Returning for Adapter {
                type Error = CustomError;
                type Transformed = Event;
            }
        };

        assert_eq!(
            super::derive(input).unwrap().to_string(),
            output.to_string(),
        );
    }

    #[test]
    fn derives_impl_with_default_infallible_error() {
        let input = parse_quote! {
            #[adapter(into = Event)]
            struct Adapter;
        };

        let output = quote! {
            #[automatically_derived]
            impl ::arcana::es::event::adapter::Returning for Adapter {
                type Error = ::std::convert::Infallible;
                type Transformed = Event;
            }
        };

        assert_eq!(
            super::derive(input).unwrap().to_string(),
            output.to_string(),
        );
    }

    #[test]
    fn derives_impl_with_generics() {
        let input = parse_quote! {
            #[adapter(transformed = Event, err = CustomError)]
            struct Adapter<T>(T);
        };

        let output = quote! {
            #[automatically_derived]
            impl<T> ::arcana::es::event::adapter::Returning for Adapter<T> {
                type Error = CustomError;
                type Transformed = Event;
            }
        };

        assert_eq!(
            super::derive(input).unwrap().to_string(),
            output.to_string(),
        );
    }

    #[test]
    fn transformed_arg_is_required() {
        let input = parse_quote! {
            #[adapter(error = CustomError)]
            struct Adapter;
        };

        let err = super::derive(input).unwrap_err();

        assert_eq!(
            err.to_string(),
            "either `into` or `transformed` argument of `#[adapter]` attribute \
             is expected to be present, but is absent",
        );
    }
}
