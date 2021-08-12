use proc_macro2::TokenStream;
use proc_macro_error::abort;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned as _,
    token,
};

use crate::common::{parsing::err, OptionExt as _};

/// Name of the derived trait.
const TRAIT_NAME: &str = "Event";

pub fn derive(input: syn::DeriveInput) -> TokenStream {
    if !matches!(&input.data, syn::Data::Struct(_)) {
        abort!(input.span(), "Only structs can derive {}", TRAIT_NAME);
    }
    derive_struct(input)
}

pub fn derive_struct(input: syn::DeriveInput) -> TokenStream {
    let ty = &input.ident;
    let (impl_generics, ty_generics, where_clause) =
        input.generics.split_for_impl();

    quote! {
        #[automatically_derived]
        impl#impl_generics ::arcana::es::event::Event for #ty#ty_generics
        #where_clause
        {
            #[inline]
            fn fqn(&self) -> ::arcana::es::event::Fqn {
                <Self as ::arcana::es::event::Typed>::FQN
            }

            #[inline]
            fn revision(&self) -> ::arcana::es::event::Revision {
                <Self as ::arcana::es::event::Typed>::REVISION
            }
        }

        #[automatically_derived]
        impl#impl_generics ::arcana::es::event::Typed for #ty#ty_generics
        #where_clause
        {
            const FQN: ::arcana::es::event::Fqn = #fqn;
            const REVISION: ::arcana::es::event::Revision =
                unsafe { Revision::new_unchecked(#revision) };
        }
    }
}

#[derive(Debug, Default)]
struct Attrs {
    fqn: Option<syn::LitStr>,
    revision: Option<syn::LitInt>,
}

impl Parse for Attrs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut output = Self::default();

        while !input.is_empty() {
            let ident = input.parse::<syn::Ident>()?;
            match ident.to_string().as_str() {
                "fqn" => {
                    input.parse::<token::Eq>()?;
                    let fqn = input.parse::<syn::LitStr>()?;
                    output
                        .fqn
                        .replace(fqn)
                        .none_or_else(|_| err::dup_attr_arg(&ident))?;
                }
                "rev" | "revision" => {
                    input.parse::<token::Eq>()?;
                    let revision = input.parse::<syn::LitInt>()?;
                    output
                        .revision
                        .replace(revision)
                        .none_or_else(|_| err::dup_attr_arg(&ident))?;
                }
                name => {
                    return Err(err::unknown_attr_arg(&ident, name));
                }
            }
        }

        Ok(output)
    }
}

impl Attrs {
    /// Tries to merge two [`Attrs`] sets into a single one, reporting about
    /// duplicates, if any.
    fn try_merge(self, mut another: Self) -> syn::Result<Self> {
        Ok(Self {
            name: try_merge_opt!(name: self, another),
            description: try_merge_opt!(description: self, another),
            context: try_merge_opt!(context: self, another),
            scalar: try_merge_opt!(scalar: self, another),
            external_resolvers: try_merge_hashmap!(
                external_resolvers: self, another => span_joined
            ),
            is_internal: self.is_internal || another.is_internal,
        })
    }
}
