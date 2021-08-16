//! TODO

use std::convert::TryFrom;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{spanned::Spanned as _, Result};
use synthez::{ParseAttrs, ToTokens};

/// Derives `arcana::VersionedEvent` for struct.
pub(crate) fn derive(input: TokenStream) -> Result<TokenStream> {
    let input = syn::parse2::<syn::DeriveInput>(input)?;
    let definitions = Definitions::try_from(input)?;

    Ok(quote! { #definitions })
}

#[derive(ToTokens)]
#[to_tokens(append(impl_from, unique_event_type_and_ver))]
struct Definitions {
    ident: syn::Ident,
    generics: syn::Generics,
    event_type: syn::LitStr,
    event_ver: syn::LitInt,
}

impl Definitions {
    fn impl_from(&self) -> TokenStream {
        let name = &self.ident;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        let (event_type, event_ver) = (&self.event_type, &self.event_ver);

        quote! {
            #[automatically_derived]
            impl #impl_generics ::arcana::VersionedEvent for
                #name #ty_generics #where_clause
            {
                #[inline(always)]
                fn event_type() -> &'static str {
                    #event_type
                }

                #[inline(always)]
                fn ver() -> u16 {
                    #event_ver
                }
            }
        }
    }

    fn unique_event_type_and_ver(&self) -> TokenStream {
        let name = &self.ident;
        let (impl_generics, ty_generics, where_clause) = self.generics.split_for_impl();
        let (event_type, event_ver) = (&self.event_type, &self.event_ver);
        let max = super::MAX_UNIQUE_EVENTS;

        quote! {
            impl #impl_generics #name #ty_generics #where_clause {
                ::arcana::unique_event_type_and_ver_for_struct!(
                    #max, #event_type, #event_ver
                );
            }
        }
    }
}

impl TryFrom<syn::DeriveInput> for Definitions {
    type Error = syn::Error;

    fn try_from(input: syn::DeriveInput) -> Result<Self> {
        if !matches!(input.data, syn::Data::Struct(..)) {
            return Err(syn::Error::new(input.span(), "Expected struct"));
        }

        let attrs: Attrs = Attrs::parse_attrs("event", &input)?;
        let (event_type, event_ver) = match (attrs.r#type, attrs.version) {
            (Some(event_type), Some(event_ver)) => (event_type, event_ver),
            _ => {
                return Err(syn::Error::new_spanned(
                    input,
                    "`type` and `version` arguments expected",
                ))
            }
        };

        Ok(Self {
            ident: input.ident,
            generics: input.generics,
            event_type,
            event_ver,
        })
    }
}

#[derive(Default, ParseAttrs)]
struct Attrs {
    #[parse(value)]
    r#type: Option<syn::LitStr>,

    #[parse(value)]
    version: Option<syn::LitInt>,
}

#[cfg(test)]
mod spec {
    use super::{derive, quote};

    #[test]
    fn derives_struct_impl() {
        let input = syn::parse_quote! {
            #[event(type = "event", version = 1)]
            struct Event;
        };

        let output = quote! {
            impl Event {
                pub const EVENT_TYPE: &'static str = "event";
                pub const EVENT_VER: u16 = 1;
            }

            #[automatically_derived]
            impl ::arcana::VersionedEvent for Event {
                #[inline(always)]
                fn event_type() -> &'static str {
                    Self::EVENT_TYPE
                }

                #[inline(always)]
                fn ver() -> u16 {
                    Self::EVENT_VER
                }
            }
        };

        assert_eq!(derive(input).unwrap().to_string(), output.to_string());
    }
}
