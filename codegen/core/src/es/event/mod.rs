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
/// - If `input` isn't a Rust enum definition;
/// - If some enum variant is not a single-field tuple struct;
/// - If failed to parse [`VariantAttrs`].
pub fn derive(input: TokenStream) -> syn::Result<TokenStream> {
    let input = syn::parse2::<syn::DeriveInput>(input)?;
    let definition = Definition::try_from(input)?;

    Ok(quote! { #definition })
}

/// Helper attributes of `#[derive(Event)]` macro placed on an enum variant.
#[derive(Debug, Default, ParseAttrs)]
pub struct VariantAttrs {
    /// Indicator whether to ignore this enum variant for code generation.
    #[parse(ident, alias = skip)]
    pub ignore: Option<syn::Ident>,
}

/// Representation of an enum implementing [`Event`], used for code generation.
///
/// [`Event`]: arcana_core::es::event::Event
#[derive(Debug, ToTokens)]
#[to_tokens(append(impl_event, gen_uniqueness_glue_code))]
pub struct Definition {
    /// [`syn::Ident`](struct@syn::Ident) of this enum's type.
    pub ident: syn::Ident,

    /// [`syn::Generics`] of this Enum's type.
    pub generics: syn::Generics,

    /// Single-[`Field`] [`Variant`]s of this enum to consider in code
    /// generation.
    ///
    /// [`Field`]: syn::Field
    /// [`Variant`]: syn::Variant
    pub variants: Vec<syn::Variant>,

    /// Indicator whether this enum has variants marked with `#[event(ignore)]`
    /// attribute.
    pub has_ignored_variants: bool,
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
            .filter_map(|v| Self::parse_variant(v).transpose())
            .collect::<syn::Result<_>>()?;
        let has_ignored_variants = variants.len() < data.variants.len();

        Ok(Self {
            ident: input.ident,
            generics: input.generics,
            variants,
            has_ignored_variants,
        })
    }
}

impl Definition {
    /// Parses and validates [`syn::Variant`] its [`VariantAttrs`].
    ///
    /// # Errors
    ///
    /// - If [`VariantAttrs`] failed to parse.
    /// - If [`syn::Variant`] doesn't have exactly one unnamed 1 [`syn::Field`]
    ///   and is not ignored.
    fn parse_variant(
        variant: &syn::Variant,
    ) -> syn::Result<Option<syn::Variant>> {
        let attrs = VariantAttrs::parse_attrs("event", variant)?;
        if attrs.ignore.is_some() {
            return Ok(None);
        }

        if variant.fields.len() != 1 {
            return Err(syn::Error::new(
                variant.span(),
                "enum variants must have exactly 1 field",
            ));
        }
        if !matches!(variant.fields, syn::Fields::Unnamed(_)) {
            return Err(syn::Error::new(
                variant.span(),
                "only tuple struct enum variants allowed",
            ));
        }

        Ok(Some(variant.clone()))
    }

    /// Generates code to derive [`Event`][0] trait, by simply matching over
    /// each enum variant, which is expected to be itself an [`Event`]
    /// implementer.
    ///
    /// [`Event`]: arcana_core::es::event::Event
    #[must_use]
    pub fn impl_event(&self) -> TokenStream {
        let ty = &self.ident;
        let (impl_gens, ty_gens, where_clause) = self.generics.split_for_impl();

        let var = self.variants.iter().map(|v| &v.ident);

        let unreachable_arm = self.has_ignored_variants.then(|| {
            quote! {
                _ => unreachable!(),
            }
        });

        quote! {
            #[automatically_derived]
            impl #impl_gens ::arcana::es::Event for #ty#ty_gens #where_clause {
                fn name(&self) -> ::arcana::es::event::Name {
                    match self {
                        #( Self::#var(f) => ::arcana::es::Event::name(f), )*
                        #unreachable_arm
                    }
                }

                fn version(&self) -> ::arcana::es::event::Version {
                    match self {
                        #( Self::#var(f) => ::arcana::es::Event::version(f), )*
                        #unreachable_arm
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
    pub fn gen_uniqueness_glue_code(&self) -> TokenStream {
        let ty = &self.ident;
        let (impl_gens, ty_gens, where_clause) = self.generics.split_for_impl();

        let var_ty =
            self.variants.iter().flat_map(|v| &v.fields).map(|f| &f.ty);

        quote! {
            #[automatically_derived]
            #[doc(hidden)]
            impl #impl_gens ::arcana::codegen::UniqueEvents for #ty#ty_gens
                 #where_clause
            {
                #[doc(hidden)]
                const COUNT: usize =
                    #( <#var_ty as ::arcana::codegen::UniqueEvents>::COUNT )+*;
            }

            #[automatically_derived]
            #[doc(hidden)]
            impl #impl_gens #ty#ty_gens #where_clause {
                #[doc(hidden)]
                pub const fn __arcana_events() -> [
                    (&'static str, u16);
                    <Self as ::arcana::codegen::UniqueEvents>::COUNT
                ] {
                    let mut res = [
                        ("", 0);
                        <Self as ::arcana::codegen::UniqueEvents>::COUNT,
                    ];

                    let mut i = 0;
                    #({
                        let events = <#var_ty>::__arcana_events();
                        let mut j = 0;
                        while j < events.len() {
                            res[i] = events[j];
                            j += 1;
                            i += 1;
                        }
                    })*

                    res
                }

                ::arcana::codegen::sa::const_assert!(
                    !::arcana::codegen::unique_events::has_duplicates(
                        Self::__arcana_events()
                    )
                );
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
