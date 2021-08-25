//! `#[derive(Event)]` macro implementation.

pub mod versioned;

use std::convert::TryFrom;

use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;
use synthez::{ParseAttrs, ToTokens};

/// Expands `#[derive(Event)]` macro.
///
/// # Errors
///
/// - If `input` isn't an `enum`;
/// - If `enum` variant consist not from single event;
/// - If failed to parse [`VariantAttrs`].
pub fn derive(input: TokenStream) -> syn::Result<TokenStream> {
    let input = syn::parse2::<syn::DeriveInput>(input)?;
    let definitions = Definition::try_from(input)?;

    Ok(quote! { #definitions })
}

/// Attributes for enum variant deriving [`Event`].
///
/// [`Event`]: arcana_core::es::Event
#[derive(Debug, Default, ParseAttrs)]
pub struct VariantAttrs {
    /// If present, [`Event`] impl and uniqueness check will be skipped for
    /// particular enum variant.
    ///
    /// [`Event`]: arcana_core::es::Event
    #[parse(ident, alias = ignore)]
    pub skip: Option<syn::Ident>,
}

/// Definition of [`Event`] derive macro.
///
/// [`Event`]: arcana_core::es::Event
#[derive(Debug, ToTokens)]
#[to_tokens(append(impl_from, unique_event_name_and_ver))]
pub struct Definition {
    /// Enum's [`Ident`].
    ///
    /// [`Ident`]: struct@syn::Ident
    pub ident: syn::Ident,

    /// Enum's [`Generics`].
    ///
    /// [`Generics`]: syn::Generics
    pub generics: syn::Generics,

    /// Enum's [`Variant`]s alongside with parsed [`VariantAttrs`].
    ///
    /// Every [`Variant`] should have exactly 1 [`Field`] in case they are not
    /// marked with `#[event(skip)]` attribute.
    ///
    /// [`Field`]: syn::Field
    /// [`Variant`]: syn::Variant
    pub variants: Vec<(syn::Variant, VariantAttrs)>,
}

impl TryFrom<syn::DeriveInput> for Definition {
    type Error = syn::Error;

    fn try_from(input: syn::DeriveInput) -> syn::Result<Self> {
        let data = if let syn::Data::Enum(data) = &input.data {
            data
        } else {
            return Err(syn::Error::new(
                input.span(),
                "expected enum only, \
                 consider using `arcana::es::event::Versioned` for structs",
            ));
        };

        let variants = data
            .variants
            .iter()
            .map(|variant| {
                let attrs = VariantAttrs::parse_attrs("event", variant)?;

                if variant.fields.len() != 1 && attrs.skip.is_none() {
                    return Err(syn::Error::new(
                        variant.span(),
                        "enum variants must have exactly 1 field",
                    ));
                }

                Ok((variant.clone(), attrs))
            })
            .collect::<syn::Result<_>>()?;

        Ok(Self {
            ident: input.ident,
            generics: input.generics,
            variants,
        })
    }
}

impl Definition {
    /// Generates code to derive [`Event`] by simply matching over every enum
    /// variant, which is expected to be itself [`Event`] deriver.
    ///
    /// # Panics
    ///
    /// If some enum [`Variant`]s don't have exactly 1 [`Field`] and not marked
    /// with `#[event(skip)]`. Checked by [`TryFrom`] impl for [`Definition`].
    ///
    /// [`Event`]: arcana_core::es::event::Event
    /// [`Field`]: syn::Field
    /// [`Variant`]: syn::Variant
    #[must_use]
    pub fn impl_from(&self) -> TokenStream {
        let name = &self.ident;
        let (impl_generics, ty_generics, where_clause) =
            self.generics.split_for_impl();
        let (event_names, event_versions): (TokenStream, TokenStream) = self
            .variants
            .iter()
            .filter_map(|(variant, attrs)| {
                if attrs.skip.is_some() {
                    return None;
                }

                let name = &variant.ident;

                let generate_variant = |func: TokenStream| match &variant.fields
                {
                    syn::Fields::Named(named) => {
                        let field = &named.named.iter().next().unwrap().ident;
                        quote! {
                            Self::#name { #field } => {
                                ::arcana::es::Event::#func(#field)
                            }
                        }
                    }
                    syn::Fields::Unnamed(_) => {
                        quote! {
                            Self::#name(inner) => {
                                ::arcana::es::Event::#func(inner)
                            }
                        }
                    }
                    syn::Fields::Unit => unreachable!(),
                };

                Some((
                    generate_variant(quote! { name }),
                    generate_variant(quote! { version }),
                ))
            })
            .unzip();

        let unreachable_for_skip = self
            .variants
            .iter()
            .any(|(_, attr)| attr.skip.is_some())
            .then(|| quote! { _ => unreachable!()});

        quote! {
            #[automatically_derived]
            impl #impl_generics ::arcana::es::Event for
                #name #ty_generics #where_clause
            {
                fn name(&self) -> ::arcana::es::event::Name {
                    match self {
                        #event_names
                        #unreachable_for_skip
                    }
                }

                fn version(&self) -> ::arcana::es::event::Version {
                    match self {
                        #event_versions
                        #unreachable_for_skip
                    }
                }
            }
        }
    }

    /// Generates functions, that returns array composed from arrays of all enum
    /// variants.
    ///
    /// Checks uniqueness of all [`Event::name`][0]s and [`Event::version`][1]s.
    ///
    /// # Panics
    ///
    /// If some enum [`Variant`]s don't have exactly 1 [`Field`] and not marked
    /// with `#[event(skip)]`. Checked by [`TryFrom`] impl  for [`Definition`].
    ///
    /// [0]: arcana_core::es::event::Event::name()
    /// [1]: arcana_core::es::event::Event::version()
    /// [`Field`]: syn::Field
    /// [`Variant`]: syn::Variant
    #[must_use]
    pub fn unique_event_name_and_ver(&self) -> TokenStream {
        let name = &self.ident;
        let (impl_generics, ty_generics, where_clause) =
            self.generics.split_for_impl();
        let (event_sizes, event_array_population): (
            Vec<TokenStream>,
            TokenStream,
        ) = self
            .variants
            .iter()
            .filter_map(|(variant, attr)| {
                attr.skip.is_none().then(|| {
                    let ty = &variant.fields.iter().next().unwrap().ty;
                    (
                        quote! {
                            <#ty as ::arcana::codegen::UniqueEvents>::COUNT
                        },
                        quote! {{
                            let ev = #ty::__arcana_events();
                            let mut local = 0;
                            while local < ev.len() {
                                res[global] = ev[local];
                                local += 1;
                                global += 1;
                            }
                        }},
                    )
                })
            })
            .unzip();

        let event_sizes = event_sizes
            .into_iter()
            .fold(None, |acc, size| {
                Some(acc.map(|acc| quote! { #acc + #size }).unwrap_or(size))
            })
            .unwrap_or(quote! { 1 });

        quote! {
            #[automatically_derived]
            impl #impl_generics ::arcana::codegen::UniqueEvents for
                #name #ty_generics #where_clause
            {
                const COUNT: usize = #event_sizes;
            }

            impl #impl_generics #name #ty_generics #where_clause {
                #[automatically_derived]
                #[doc(hidden)]
                pub const fn __arcana_events() -> [
                    (&'static str, u16);
                    <Self as ::arcana::codegen::UniqueEvents>::COUNT
                ] {
                    let mut res = [
                        ("", 0);
                        <Self as ::arcana::codegen::UniqueEvents>::COUNT
                    ];

                    let mut global = 0;

                    #event_array_population

                    res
                }
            }

            ::arcana::codegen::sa::const_assert!(
                !::arcana::codegen::unique_events::has_duplicates(
                    #name::__arcana_events()
                )
            );
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
                Event1(EventUnnamed),
                Event2 {
                    event: EventNamed,
                }
            }
        };

        let output = quote! {
            #[automatically_derived]
            impl ::arcana::es::Event for Event {
                fn name(&self) -> ::arcana::es::event::Name {
                    match self {
                        Self::Event1(inner) => {
                            ::arcana::es::Event::name(inner)
                        }
                        Self::Event2 { event } => {
                            ::arcana::es::Event::name(event)
                        }
                    }
                }

                fn version(&self) -> ::arcana::es::event::Version {
                    match self {
                        Self::Event1(inner) => {
                            ::arcana::es::Event::version(inner)
                        }
                        Self::Event2 { event } => {
                            ::arcana::es::Event::version(event)
                        }
                    }
                }
            }

            #[automatically_derived]
            impl ::arcana::codegen::UniqueEvents for Event {
                const COUNT: usize =
                    <EventUnnamed as ::arcana::codegen::UniqueEvents>::COUNT +
                    <EventNamed as ::arcana::codegen::UniqueEvents>::COUNT;
            }

            impl Event {
                #[automatically_derived]
                #[doc(hidden)]
                pub const fn __arcana_events() -> [
                    (&'static str, u16);
                    <Self as ::arcana::codegen::UniqueEvents>::COUNT
                ] {
                    let mut res = [
                        ("", 0);
                        <Self as ::arcana::codegen::UniqueEvents>::COUNT
                    ];

                    let mut global = 0;

                    {
                        let ev = EventUnnamed::__arcana_events();
                        let mut local = 0;
                        while local < ev.len() {
                            res[global] = ev[local];
                            local += 1;
                            global += 1;
                        }
                    }

                    {
                        let ev = EventNamed::__arcana_events();
                        let mut local = 0;
                        while local < ev.len() {
                            res[global] = ev[local];
                            local += 1;
                            global += 1;
                        }
                    }

                    res
                }
            }

            ::arcana::codegen::sa::const_assert!(
                !::arcana::codegen::unique_events::has_duplicates(
                    Event::__arcana_events()
                )
            );
        };

        assert_eq!(derive(input).unwrap().to_string(), output.to_string());
    }

    #[test]
    fn skip_unique_check_on_variant() {
        let input_skip = syn::parse_quote! {
            enum Event {
                Event1(EventUnnamed),
                Event2 {
                    event: EventNamed,
                },
                #[event(skip)]
                #[doc(hidden)]
                _NonExhaustive
            }
        };

        let input_ignore = syn::parse_quote! {
            enum Event {
                Event1(EventUnnamed),
                Event2 {
                    event: EventNamed,
                },
                #[event(ignore)]
                #[doc(hidden)]
                _NonExhaustive
            }
        };

        let output = quote! {
            #[automatically_derived]
            impl ::arcana::es::Event for Event {
                fn name(&self) -> ::arcana::es::event::Name {
                    match self {
                        Self::Event1(inner) => {
                            ::arcana::es::Event::name(inner)
                        }
                        Self::Event2 { event } => {
                            ::arcana::es::Event::name(event)
                        }
                        _ => unreachable!()
                    }
                }

                fn version(&self) -> ::arcana::es::event::Version {
                    match self {
                        Self::Event1(inner) => {
                            ::arcana::es::Event::version(inner)
                        }
                        Self::Event2 { event } => {
                            ::arcana::es::Event::version(event)
                        }
                        _ => unreachable!()
                    }
                }
            }

            #[automatically_derived]
            impl ::arcana::codegen::UniqueEvents for Event {
                const COUNT: usize =
                    <EventUnnamed as ::arcana::codegen::UniqueEvents>::COUNT +
                    <EventNamed as ::arcana::codegen::UniqueEvents>::COUNT;
            }

            impl Event {
                #[automatically_derived]
                #[doc(hidden)]
                pub const fn __arcana_events() -> [
                    (&'static str, u16);
                    <Self as ::arcana::codegen::UniqueEvents>::COUNT
                ] {
                    let mut res = [
                        ("", 0);
                        <Self as ::arcana::codegen::UniqueEvents>::COUNT
                    ];

                    let mut global = 0;

                    {
                        let ev = EventUnnamed::__arcana_events();
                        let mut local = 0;
                        while local < ev.len() {
                            res[global] = ev[local];
                            local += 1;
                            global += 1;
                        }
                    }

                    {
                        let ev = EventNamed::__arcana_events();
                        let mut local = 0;
                        while local < ev.len() {
                            res[global] = ev[local];
                            local += 1;
                            global += 1;
                        }
                    }

                    res
                }
            }

            ::arcana::codegen::sa::const_assert!(
                !::arcana::codegen::unique_events::has_duplicates(
                    Event::__arcana_events()
                )
            );
        };

        let input_skip = derive(input_skip).unwrap().to_string();
        let input_ignore = derive(input_ignore).unwrap().to_string();
        assert_eq!(input_skip, input_ignore);
        assert_eq!(input_skip, output.to_string());
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
            "enum variants must have exactly 1 field",
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
            "expected enum only, \
             consider using `arcana::es::event::Versioned` for structs",
        );
    }
}
