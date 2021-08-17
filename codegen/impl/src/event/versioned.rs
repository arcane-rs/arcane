//! Definition of `arcana::VersionedEvent` derive macro for structs.

use std::{convert::TryFrom, num::NonZeroU16};

use proc_macro2::TokenStream;
use quote::quote;
use syn::{spanned::Spanned as _, Result};
use synthez::{ParseAttrs, ToTokens};

use super::MAX_UNIQUE_EVENTS;

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
        let (impl_generics, ty_generics, where_clause) =
            self.generics.split_for_impl();
        let (event_type, event_ver) = (&self.event_type, &self.event_ver);

        quote! {
            #[automatically_derived]
            impl #impl_generics ::arcana::VersionedEvent for
                #name #ty_generics #where_clause
            {
                #[inline(always)]
                fn event_type() -> ::arcana::EventName {
                    #event_type
                }

                #[inline(always)]
                fn ver() -> ::arcana::EventVersion {
                    // This is safe, because checked by proc-macro.
                    #[allow(unsafe_code)]
                    unsafe { ::arcana::EventVersion::new_unchecked(#event_ver) }
                }
            }
        }
    }

    fn unique_event_type_and_ver(&self) -> TokenStream {
        let name = &self.ident;
        let (impl_generics, ty_generics, where_clause) =
            self.generics.split_for_impl();
        let (event_type, event_ver) = (&self.event_type, &self.event_ver);
        let max = MAX_UNIQUE_EVENTS;

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
            return Err(syn::Error::new(
                input.span(),
                "Expected struct. Consider using arcana::Event for enums",
            ));
        }

        let attrs = Attrs::parse_attrs("event", &input)?;
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

    #[parse(value, validate = parses_to_non_zero_u16)]
    version: Option<syn::LitInt>,
}

fn parses_to_non_zero_u16<'a>(
    val: impl Into<Option<&'a syn::LitInt>>,
) -> Result<()> {
    val.into()
        .map(syn::LitInt::base10_parse::<NonZeroU16>)
        .transpose()
        .map(drop)
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
                fn event_type() -> ::arcana::EventName {
                    "event"
                }

                #[inline(always)]
                fn ver() -> ::arcana::EventVersion {
                    // This is safe, because checked by proc-macro.
                    #[allow(unsafe_code)]
                    unsafe { ::arcana::EventVersion::new_unchecked(1) }
                }
            }

            impl Event {
                ::arcana::unique_event_type_and_ver_for_struct!(
                    100000usize, "event", 1
                );
            }
        };

        assert_eq!(derive(input).unwrap().to_string(), output.to_string());
    }

    #[test]
    fn type_argument_is_expected() {
        let input = syn::parse_quote! {
            #[event(version = 1)]
            struct Event;
        };

        let error = derive(input).unwrap_err();

        assert_eq!(
            format!("{}", error),
            "`type` and `version` arguments expected",
        );
    }

    #[test]
    fn version_argument_is_expected() {
        let input = syn::parse_quote! {
            #[event(type = "event")]
            struct Event;
        };

        let error = derive(input).unwrap_err();

        assert_eq!(
            format!("{}", error),
            "`type` and `version` arguments expected",
        );
    }

    #[test]
    fn errors_on_negative_version() {
        let input = syn::parse_quote! {
            #[event(type = "event", version = -1)]
            struct Event;
        };

        let error = derive(input).unwrap_err();

        assert_eq!(format!("{}", error), "invalid digit found in string",);
    }

    #[test]
    fn errors_on_zero_version() {
        let input = syn::parse_quote! {
            #[event(type = "event", version = 0)]
            struct Event;
        };

        let error = derive(input).unwrap_err();

        assert_eq!(
            format!("{}", error),
            "number would be zero for non-zero type",
        );
    }

    #[test]
    fn errors_on_too_big_version() {
        let input = syn::parse_quote! {
            #[event(type = "event", version = 4294967295)]
            struct Event;
        };

        let error = derive(input).unwrap_err();

        assert_eq!(
            format!("{}", error),
            "number too large to fit in target type",
        );
    }

    #[test]
    fn errors_on_enum() {
        let input = syn::parse_quote! {
            #[event(type = "event", version = 1)]
            enum Event {
                Event1(Event1),
            }
        };

        let error = derive(input).unwrap_err();

        assert_eq!(
            format!("{}", error),
            "Expected struct. Consider using arcana::Event for enums",
        );
    }
}
