//! TODO

use proc_macro2::TokenStream;
use quote::quote;
use syn::Result;
use synthez::ParseAttrs;

/// Derives `serde::Deserialize` for `arcana::VersionedEvent`.
pub(crate) fn derive(input: TokenStream) -> Result<TokenStream> {
    let input: syn::DeriveInput = syn::parse2(input)?;
    let attrs: Attrs = Attrs::parse_attrs("event", &input)?;

    match (attrs.r#type, attrs.version) {
        (Some(event_type), Some(event_version)) => {
            let name = &input.ident;
            let (impl_generics, ty_generics, where_clause) = input.generics.split_for_impl();

            Ok(quote! {
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
                        #event_version
                    }
                }
            })
        }
        _ => Err(syn::Error::new_spanned(
            input,
            "`type` and `version` arguments expected",
        )),
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
            #[automatically_derived]
            impl ::arcana::VersionedEvent for Event {
                #[inline(always)]
                fn event_type() -> &'static str {
                    "event"
                }

                #[inline(always)]
                fn ver() -> u16 {
                    1
                }
            }
        };

        assert_eq!(derive(input).unwrap().to_string(), output.to_string());
    }
}
