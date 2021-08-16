//! TODO

pub(crate) mod versioned;

use std::convert::TryFrom;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{punctuated::Punctuated, spanned::Spanned as _, Result};
use synthez::ToTokens;

const MAX_UNIQUE_EVENTS: usize = 100000;

/// Derives `arcana::Event` for enum.
pub(crate) fn derive(input: TokenStream) -> Result<TokenStream> {
    let input: syn::DeriveInput = syn::parse2(input)?;
    let definitions = EnumDefinitions::try_from(input)?;

    Ok(quote! { #definitions })
}

#[derive(ToTokens)]
#[to_tokens(append(impl_from, unique_event_type_and_ver))]
struct EnumDefinitions {
    ident: syn::Ident,
    generics: syn::Generics,
    variants: Punctuated<syn::Variant, syn::Token![,]>,
}

impl EnumDefinitions {
    fn impl_from(&self) -> TokenStream {
        let name = &self.ident;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        let (event_types, event_vers): (TokenStream, TokenStream) = self
            .variants
            .iter()
            .map(|variant| {
                let name = &variant.ident;

                let generate_variant = |func: TokenStream| match &variant.fields {
                    syn::Fields::Named(named) => {
                        let field = &named.named.iter().next().unwrap().ident;
                        quote! {
                            Self::#name { #field } => {
                                ::arcana::Event::#func(#field)
                            }
                        }
                    }
                    syn::Fields::Unnamed(_) => {
                        quote! {
                            Self::#name(inner) => {
                                ::arcana::Event::#func(inner)
                            }
                        }
                    }
                    syn::Fields::Unit => unreachable!(),
                };

                let (ty, ver) = (
                    generate_variant(quote! { event_type }),
                    generate_variant(quote! { ver }),
                );

                (quote! { #ty }, quote! { #ver })
            })
            .unzip();

        quote! {
            #[automatically_derived]
            impl #impl_generics ::arcana::Event for
                #name #ty_generics #where_clause
            {
                #[inline(always)]
                fn event_type(&self) -> &'static str {
                    match self {
                        #event_types
                    }
                }

                #[inline(always)]
                fn ver(&self) -> u16 {
                    match self {
                        #event_vers
                    }
                }
            }
        }
    }

    fn unique_event_type_and_ver(&self) -> TokenStream {
        let name = &self.ident;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        let event_variants = self
            .variants
            .iter()
            .map(|variant| {
                let ty = &variant.fields.iter().next().unwrap().ty;
                quote! { #ty, }
            })
            .collect::<TokenStream>();
        let max = MAX_UNIQUE_EVENTS;

        quote! {
            impl #impl_generics #name #ty_generics #where_clause {
                ::arcana::unique_event_type_and_ver_for_enum!(
                    #max, #event_variants
                );
            }

            arcana::unique_event_type_and_ver_check!(#name);
        }
    }
}

impl TryFrom<syn::DeriveInput> for EnumDefinitions {
    type Error = syn::Error;

    fn try_from(input: syn::DeriveInput) -> Result<Self> {
        let data = if let syn::Data::Enum(data) = &input.data {
            data
        } else {
            return Err(syn::Error::new(input.span(), "Expected enum"));
        };

        for variant in &data.variants {
            if variant.fields.len() != 1 {
                return Err(syn::Error::new(
                    variant.span(),
                    "Enum variants must have exactly 1 field",
                ));
            }
        }

        Ok(Self {
            ident: input.ident,
            generics: input.generics,
            variants: data.variants.clone(),
        })
    }
}

#[cfg(test)]
mod spec {
    use super::{derive, quote};

    #[test]
    fn derives_enum_impl() {
        let input = syn::parse_quote! {
            enum Event {
                Event1(EventUnnamend),
                Event2 {
                    event: EventNamed,
                }
            }
        };

        let output = quote! {
            #[automatically_derived]
            impl ::arcana::Event for Event {
                #[inline(always)]
                fn event_type(&self) -> &'static str {
                    match self {
                        Self::Event1(inner) => {
                            ::arcana::Event::event_type(inner)
                        }
                        Self::Event2 { event } => {
                            ::arcana::Event::event_type(event)
                        }
                    }
                }

                #[inline(always)]
                fn ver(&self) -> u16 {
                    match self {
                        Self::Event1(inner) => {
                            ::arcana::Event::ver(inner)
                        }
                        Self::Event2 { event } => {
                            ::arcana::Event::ver(event)
                        }
                    }
                }
            }
        };

        assert_eq!(derive(input).unwrap().to_string(), output.to_string());
    }
}
