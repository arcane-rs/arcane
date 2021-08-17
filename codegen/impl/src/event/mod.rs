//! Definition of `arcana::Event` derive macro for enums.

pub(crate) mod versioned;

use std::{convert::TryFrom, result::Result as StdResult, str::FromStr};

use proc_macro2::{Span, TokenStream};
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    spanned::Spanned,
    Result,
};
use synthez::{ParseAttrs, ToTokens};

const MAX_UNIQUE_EVENTS: usize = 100_000;

/// Derives `arcana::Event` for enum.
pub(crate) fn derive(input: TokenStream) -> Result<TokenStream> {
    let input: syn::DeriveInput = syn::parse2(input)?;
    let definitions = Definitions::try_from(input)?;

    Ok(quote! { #definitions })
}

#[derive(ToTokens)]
#[to_tokens(append(impl_from, unique_event_type_and_ver))]
struct Definitions {
    ident: syn::Ident,
    generics: syn::Generics,
    variants: Vec<(syn::Variant, Attrs)>,
    attrs: Attrs,
}

impl Definitions {
    fn impl_from(&self) -> TokenStream {
        let name = &self.ident;
        let (impl_generics, ty_generics, where_clause) =
            self.generics.split_for_impl();
        let (event_types, event_versions): (TokenStream, TokenStream) = self
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
                    generate_variant(quote! { event_type }),
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
                fn event_type(&self) -> &'static str {
                    match self {
                        #event_types
                    }
                }

                #[inline(always)]
                fn ver(&self) -> u16 {
                    match self {
                        #event_versions
                    }
                }
            }
        }
    }

    fn unique_event_type_and_ver(&self) -> TokenStream {
        if self.attrs.skip_check_unique_type_and_ver() {
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
                (!attr.skip_check_unique_type_and_ver()).then(|| {
                    let ty = &variant.fields.iter().next().unwrap().ty;
                    quote! { #ty, }
                })
            })
            .collect::<TokenStream>();

        quote! {
            impl #impl_generics #name #ty_generics #where_clause {
                ::arcana::unique_event_type_and_ver_for_enum!(
                    #max, #event_variants
                );
            }

            ::arcana::unique_event_type_and_ver_check!(#name);
        }
    }
}

impl TryFrom<syn::DeriveInput> for Definitions {
    type Error = syn::Error;

    fn try_from(input: syn::DeriveInput) -> Result<Self> {
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
            .collect::<Result<_>>()?;

        Ok(Self {
            ident: input.ident,
            generics: input.generics,
            variants,
            attrs,
        })
    }
}

#[derive(Default, ParseAttrs)]
struct Attrs {
    #[parse(value)]
    skip: Option<Spanning<SkipAttr>>,
}

impl Attrs {
    fn skip_check_unique_type_and_ver(&self) -> bool {
        matches!(
            self.skip.as_ref().map(|sp| sp.item),
            Some(SkipAttr::CheckUniqueTypeAndVer),
        )
    }
}

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

#[derive(Clone, Copy, Debug)]
enum SkipAttr {
    CheckUniqueTypeAndVer,
}

impl Parse for Spanning<SkipAttr> {
    fn parse(input: ParseStream<'_>) -> Result<Self> {
        let ident = syn::Ident::parse(input)?;
        Ok(Spanning {
            item: SkipAttr::from_str(&ident.to_string())
                .map_err(|err| syn::Error::new(ident.span(), err))?,
            span: ident.span(),
        })
    }
}

impl FromStr for SkipAttr {
    type Err = &'static str;

    fn from_str(s: &str) -> StdResult<Self, Self::Err> {
        match s {
            "check_unique_type_and_ver" => Ok(Self::CheckUniqueTypeAndVer),
            _ => {
                Err("Unknown value. Allowed values: check_unique_type_and_ver")
            }
        }
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

            impl Event {
                ::arcana::unique_event_type_and_ver_for_enum!(
                    100000usize, EventUnnamend, EventNamed,
                );
            }

            ::arcana::unique_event_type_and_ver_check!(Event);
        };

        assert_eq!(derive(input).unwrap().to_string(), output.to_string());
    }

    #[test]
    fn skip_unique_check_on_container() {
        let input = syn::parse_quote! {
            #[event(skip(check_unique_type_and_ver))]
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

    #[test]
    fn skip_unique_check_on_variant() {
        let input = syn::parse_quote! {
            enum Event {
                #[event(skip(check_unique_type_and_ver))]
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

            impl Event {
                ::arcana::unique_event_type_and_ver_for_enum!(
                    100000usize, EventNamed,
                );
            }

            ::arcana::unique_event_type_and_ver_check!(Event);
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
            "Unknown value. Allowed values: check_unique_type_and_ver",
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
