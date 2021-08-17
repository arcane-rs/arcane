//! Definition of [`Event`] derive macro for enums.
//!
//! [`Event`]: arcana_core::Event

pub(crate) mod versioned;

use std::{convert::TryFrom, str::FromStr as _};

use proc_macro2::{Span, TokenStream};
use quote::quote;
use strum::{EnumString, EnumVariantNames, VariantNames as _};
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
};
use synthez::{ParseAttrs, ToTokens};

const MAX_UNIQUE_EVENTS: usize = 100_000;

/// Derives [`Event`] for enum.
///
/// [`Event`]: arcana_core::Event
pub(crate) fn derive(input: TokenStream) -> syn::Result<TokenStream> {
    let input: syn::DeriveInput = syn::parse2(input)?;
    let definitions = Definitions::try_from(input)?;

    Ok(quote! { #definitions })
}

/// Attributes for [`Event`] derive macro.
///
/// [`Event`]: arcana_core::Event
#[derive(Default, ParseAttrs)]
struct Attrs {
    /// `#[event(skip(...))` attribute.
    #[parse(value)]
    skip: Option<Spanning<SkipAttr>>,
}

impl Attrs {
    /// Checks whether variant or whole container shouldn't be checked for
    /// [`Event::name()`] and [`Event::ver()`] uniqueness.
    ///
    /// [`Event::name()`]: arcana_core::Event::name()
    /// [`Event::ver()`]: arcana_core::Event::ver()
    fn skip_check_unique_name_and_ver(&self) -> bool {
        matches!(
            self.skip.as_ref().map(|sp| sp.item),
            Some(SkipAttr::CheckUniqueNameAndVer),
        )
    }
}

/// Wrapper for storing [`Span`].
///
/// We don't use one from [`synthez`], as we can't derive [`Parse`] with our `T`
/// inside.
#[derive(Clone, Debug)]
struct Spanning<T> {
    item: T,
    span: Span,
}

impl<T> Spanned for Spanning<T> {
    fn span(&self) -> Span {
        self.span
    }
}

/// Inner value for `#[event(skip(...))]` attribute.
#[derive(Clone, Copy, Debug, EnumString, EnumVariantNames)]
#[strum(serialize_all = "snake_case")]
enum SkipAttr {
    /// Variant for skipping uniqueness check of [`Event::name()`] and
    /// [`Event::ver()`].
    ///
    /// [`Event::name()`]: arcana_core::Event::name()
    /// [`Event::ver()`]: arcana_core::Event::ver()
    CheckUniqueNameAndVer,
}

impl Parse for Spanning<SkipAttr> {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let ident = syn::Ident::parse(input)?;
        Ok(Spanning {
            item: SkipAttr::from_str(&ident.to_string()).map_err(|_| {
                syn::Error::new(
                    ident.span(),
                    &format!(
                        "Unknown value. Allowed values: {}",
                        SkipAttr::VARIANTS.join(", "),
                    ),
                )
            })?,
            span: ident.span(),
        })
    }
}

/// Definition of [`Event`] derive macro.
///
/// [`Event`]: arcana_core::Event
#[derive(ToTokens)]
#[to_tokens(append(impl_from, unique_event_name_and_ver))]
struct Definitions {
    /// Enum's [`Ident`].
    ///
    /// [`Ident`]: syn::Ident
    ident: syn::Ident,

    /// Enum's [`Generics`].
    ///
    /// [`Generics`]: syn::Generics
    generics: syn::Generics,

    /// Enum's [`Variant`]s alongside with parsed [`Attrs`].
    ///
    /// Every [`Variant`] has exactly 1 [`Field`].
    ///
    /// [`Field`]: syn::Field
    /// [`Variant`]: syn::Variant
    variants: Vec<(syn::Variant, Attrs)>,

    /// Enum's top-level [`Attrs`].
    attrs: Attrs,
}

impl Definitions {
    /// Generates code to derive [`Event`] by simply matching over every enum
    /// variant, which is expected to be itself [`Event`] deriver.
    ///
    /// [`Event`]: arcana_core::Event
    fn impl_from(&self) -> TokenStream {
        let name = &self.ident;
        let (impl_generics, ty_generics, where_clause) =
            self.generics.split_for_impl();
        let (event_names, event_versions): (TokenStream, TokenStream) = self
            .variants
            .iter()
            .map(|(variant, _)| {
                let name = &variant.ident;

                let generate_variant = |func: TokenStream| match &variant.fields
                {
                    syn::Fields::Named(named) => {
                        // Unwrapping is safe here as we checked for
                        // `.len() == 1` in TryFrom impl.
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

                (
                    generate_variant(quote! { name }),
                    generate_variant(quote! { ver }),
                )
            })
            .unzip();

        quote! {
            #[automatically_derived]
            impl #impl_generics ::arcana::Event for
                #name #ty_generics #where_clause
            {
                #[inline(always)]
                fn name(&self) -> ::arcana::EventName {
                    match self {
                        #event_names
                    }
                }

                #[inline(always)]
                fn ver(&self) -> ::arcana::EventVersion {
                    match self {
                        #event_versions
                    }
                }
            }
        }
    }

    /// Generates code, that checks uniqueness of [`Event::name()`] and
    /// [`Event::ver()`].
    ///
    /// [`Event::name()`]: arcana_core::Event::name()
    /// [`Event::ver()`]: arcana_core::Event::ver()
    fn unique_event_name_and_ver(&self) -> TokenStream {
        if self.attrs.skip_check_unique_name_and_ver() {
            return TokenStream::new();
        }

        let max = MAX_UNIQUE_EVENTS;
        let name = &self.ident;
        let (impl_generics, ty_generics, where_clause) =
            self.generics.split_for_impl();
        let event_variants = self
            .variants
            .iter()
            .filter_map(|(variant, attr)| {
                (!attr.skip_check_unique_name_and_ver()).then(|| {
                    let ty = &variant.fields.iter().next().unwrap().ty;
                    quote! { #ty, }
                })
            })
            .collect::<TokenStream>();

        quote! {
            impl #impl_generics #name #ty_generics #where_clause {
                ::arcana::unique_event_name_and_ver_for_enum!(
                    #max, #event_variants
                );
            }

            ::arcana::unique_event_name_and_ver_check!(#name);
        }
    }
}

impl TryFrom<syn::DeriveInput> for Definitions {
    type Error = syn::Error;

    fn try_from(input: syn::DeriveInput) -> syn::Result<Self> {
        let data = if let syn::Data::Enum(data) = &input.data {
            data
        } else {
            return Err(syn::Error::new(
                input.span(),
                "Expected enum. \
                 Consider using arcana::VersionedEvent for structs",
            ));
        };

        for variant in &data.variants {
            if variant.fields.len() != 1 {
                return Err(syn::Error::new(
                    variant.span(),
                    "Enum variants must have exactly 1 field",
                ));
            }
        }

        let attrs = Attrs::parse_attrs("event", &input)?;
        let variants = data
            .variants
            .iter()
            .map(|variant| {
                Ok((variant.clone(), Attrs::parse_attrs("event", variant)?))
            })
            .collect::<syn::Result<_>>()?;

        Ok(Self {
            ident: input.ident,
            generics: input.generics,
            variants,
            attrs,
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
                fn name(&self) -> ::arcana::EventName {
                    match self {
                        Self::Event1(inner) => {
                            ::arcana::Event::name(inner)
                        }
                        Self::Event2 { event } => {
                            ::arcana::Event::name(event)
                        }
                    }
                }

                #[inline(always)]
                fn ver(&self) -> ::arcana::EventVersion {
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

            impl Event {
                ::arcana::unique_event_name_and_ver_for_enum!(
                    100000usize, EventUnnamend, EventNamed,
                );
            }

            ::arcana::unique_event_name_and_ver_check!(Event);
        };

        assert_eq!(derive(input).unwrap().to_string(), output.to_string());
    }

    #[test]
    fn skip_unique_check_on_container() {
        let input = syn::parse_quote! {
            #[event(skip(check_unique_name_and_ver))]
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
                fn name(&self) -> ::arcana::EventName {
                    match self {
                        Self::Event1(inner) => {
                            ::arcana::Event::name(inner)
                        }
                        Self::Event2 { event } => {
                            ::arcana::Event::name(event)
                        }
                    }
                }

                #[inline(always)]
                fn ver(&self) -> ::arcana::EventVersion {
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

    #[test]
    fn skip_unique_check_on_variant() {
        let input = syn::parse_quote! {
            enum Event {
                #[event(skip(check_unique_name_and_ver))]
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
                fn name(&self) -> ::arcana::EventName {
                    match self {
                        Self::Event1(inner) => {
                            ::arcana::Event::name(inner)
                        }
                        Self::Event2 { event } => {
                            ::arcana::Event::name(event)
                        }
                    }
                }

                #[inline(always)]
                fn ver(&self) -> ::arcana::EventVersion {
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

            impl Event {
                ::arcana::unique_event_name_and_ver_for_enum!(
                    100000usize, EventNamed,
                );
            }

            ::arcana::unique_event_name_and_ver_check!(Event);
        };

        assert_eq!(derive(input).unwrap().to_string(), output.to_string());
    }

    #[test]
    fn errors_on_multiple_fields_in_variant() {
        let input = syn::parse_quote! {
            enum Event {
                Event1(Event1),
                Event2 {
                    event: Event2,
                    second_field: Event3,
                }
            }
        };

        let error = derive(input).unwrap_err();

        assert_eq!(
            format!("{}", error),
            "Enum variants must have exactly 1 field",
        );
    }

    #[test]
    fn errors_on_unknown_attribute_value() {
        let input = syn::parse_quote! {
            enum Event {
                #[event(skip(unknown))]
                Event1(Event1),
            }
        };

        let error = derive(input).unwrap_err();

        assert_eq!(
            format!("{}", error),
            "Unknown value. Allowed values: check_unique_name_and_ver",
        );
    }

    #[test]
    fn errors_on_struct() {
        let input = syn::parse_quote! {
            struct Event;
        };

        let error = derive(input).unwrap_err();

        assert_eq!(
            format!("{}", error),
            "Expected enum. Consider using arcana::VersionedEvent for structs",
        );
    }
}
