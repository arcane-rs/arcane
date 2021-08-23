//! Definition of [`VersionedEvent`] derive macro for structs.
//!
//! [`VersionedEvent`]: arcana_core::VersionedEvent

use std::{convert::TryFrom, num::NonZeroU16};

use proc_macro2::TokenStream;
use quote::quote;
use syn::{spanned::Spanned as _, Result};
use synthez::{ParseAttrs, ToTokens};

/// Derives [`VersionedEvent`] for struct.
///
/// [`VersionedEvent`]: arcana_core::VersionedEvent
///
/// # Errors
///
/// - If `input` isn't a `struct`;
/// - If failed to parse [`Attrs`].
pub fn derive(input: TokenStream) -> Result<TokenStream> {
    let input = syn::parse2::<syn::DeriveInput>(input)?;
    let definitions = Definitions::try_from(input)?;

    Ok(quote! { #definitions })
}

/// Attributes for [`VersionedEvent`] derive macro.
///
/// [`VersionedEvent`]: arcana_core::VersionedEvent
#[derive(Debug, Default, ParseAttrs)]
pub struct Attrs {
    /// Value for [`VersionedEvent::name()`] impl.
    ///
    /// [`VersionedEvent::name()`]: arcana_core::VersionedEvent::name()
    #[parse(value)]
    name: Option<syn::LitStr>,

    /// Value for [`VersionedEvent::ver()`] impl.
    ///
    /// [`VersionedEvent::ver()`]: arcana_core::VersionedEvent::ver()
    #[parse(value, validate = parses_to_non_zero_u16)]
    version: Option<syn::LitInt>,
}

/// If `val` is [`Some`], checks if it can be parsed to [`NonZeroU16`].
fn parses_to_non_zero_u16<'a>(
    val: impl Into<Option<&'a syn::LitInt>>,
) -> Result<()> {
    val.into()
        .map(syn::LitInt::base10_parse::<NonZeroU16>)
        .transpose()
        .map(drop)
}

/// Definition of [`VersionedEvent`] derive macro.
///
/// [`VersionedEvent`]: arcana_core::Event
#[derive(ToTokens)]
#[to_tokens(append(impl_from, unique_event_name_and_ver))]
struct Definitions {
    /// Struct's [`Ident`].
    ///
    /// [`Ident`]: syn::Ident
    ident: syn::Ident,

    /// Struct's [`Generics`].
    ///
    /// [`Generics`]: syn::Generics
    generics: syn::Generics,

    /// [`Attr::name`] from top-level struct attribute.
    event_name: syn::LitStr,

    /// [`Attr::version`] from top-level struct attribute.
    event_ver: syn::LitInt,
}

impl Definitions {
    /// Generates code to derive [`VersionedEvent`] by placing values from
    /// [`Attrs`] inside [`VersionedEvent::name()`] and
    /// [`VersionedEvent::ver()`] impls.
    ///
    /// [`VersionedEvent`]: arcana_core::VersionedEvent
    /// [`VersionedEvent::name()`]: arcana_core::VersionedEvent::name()
    /// [`VersionedEvent::ver()`]: arcana_core::VersionedEvent::ver()
    fn impl_from(&self) -> TokenStream {
        let name = &self.ident;
        let (impl_generics, ty_generics, where_clause) =
            self.generics.split_for_impl();
        let (event_name, event_ver) = (&self.event_name, &self.event_ver);

        quote! {
            #[automatically_derived]
            impl #impl_generics ::arcana::VersionedEvent for
                #name #ty_generics #where_clause
            {
                #[inline(always)]
                fn name() -> ::arcana::EventName {
                    #event_name
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

    /// Generates functions, that returns array of size 1 with
    /// [`VersionedEvent::name()`] and [`VersionedEvent::ver()`]. Used for
    /// uniqueness check.
    ///
    /// [`VersionedEvent::name()`]: arcana_core::VersionedEvent::name()
    /// [`VersionedEvent::ver()`]: arcana_core::VersionedEvent::ver()
    fn unique_event_name_and_ver(&self) -> TokenStream {
        let name = &self.ident;
        let (impl_generics, ty_generics, where_clause) =
            self.generics.split_for_impl();
        let (event_name, event_ver) = (&self.event_name, &self.event_ver);

        quote! {
            #[automatically_derived]
            impl #impl_generics ::arcana::codegen::UniqueEvents for
                #name #ty_generics #where_clause
            {
                const COUNT: usize = 1;
            }

            impl #impl_generics #name #ty_generics #where_clause {
                #[automatically_derived]
                pub const fn __arcana_events() -> [(&'static str, u16); 1] {
                    [(#event_name, #event_ver)]
                }
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
        let (event_name, event_ver) = match (attrs.name, attrs.version) {
            (Some(event_name), Some(event_ver)) => (event_name, event_ver),
            _ => {
                return Err(syn::Error::new_spanned(
                    input,
                    "`name` and `version` arguments expected",
                ))
            }
        };

        Ok(Self {
            ident: input.ident,
            generics: input.generics,
            event_name,
            event_ver,
        })
    }
}

#[cfg(test)]
mod spec {
    use super::{derive, quote};

    #[test]
    fn derives_struct_impl() {
        let input = syn::parse_quote! {
            #[event(name = "event", version = 1)]
            struct Event;
        };

        let output = quote! {
            #[automatically_derived]
            impl ::arcana::VersionedEvent for Event {
                #[inline(always)]
                fn name() -> ::arcana::EventName {
                    "event"
                }

                #[inline(always)]
                fn ver() -> ::arcana::EventVersion {
                    // This is safe, because checked by proc-macro.
                    #[allow(unsafe_code)]
                    unsafe { ::arcana::EventVersion::new_unchecked(1) }
                }
            }

            #[automatically_derived]
            impl ::arcana::codegen::UniqueEvents for Event {
                const COUNT: usize = 1;
            }

            impl Event {
                #[automatically_derived]
                pub const fn __arcana_events() -> [(&'static str, u16); 1] {
                    [("event", 1)]
                }
            }
        };

        assert_eq!(derive(input).unwrap().to_string(), output.to_string());
    }

    #[test]
    fn name_argument_is_expected() {
        let input = syn::parse_quote! {
            #[event(version = 1)]
            struct Event;
        };

        let error = derive(input).unwrap_err();

        assert_eq!(
            format!("{}", error),
            "`name` and `version` arguments expected",
        );
    }

    #[test]
    fn version_argument_is_expected() {
        let input = syn::parse_quote! {
            #[event(name = "event")]
            struct Event;
        };

        let error = derive(input).unwrap_err();

        assert_eq!(
            format!("{}", error),
            "`name` and `version` arguments expected",
        );
    }

    #[test]
    fn errors_on_negative_version() {
        let input = syn::parse_quote! {
            #[event(name = "event", version = -1)]
            struct Event;
        };

        let error = derive(input).unwrap_err();

        assert_eq!(format!("{}", error), "invalid digit found in string",);
    }

    #[test]
    fn errors_on_zero_version() {
        let input = syn::parse_quote! {
            #[event(name = "event", version = 0)]
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
            #[event(name = "event", version = 4294967295)]
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
            #[event(name = "event", version = 1)]
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
