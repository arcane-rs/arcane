//! `#[derive(Event)]` macro implementation.

pub mod transformer;
pub mod versioned;

use std::convert::TryFrom;

use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned as _;
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

    /// [`syn::Generics`] of this enum's type.
    pub generics: syn::Generics,

    /// Single-[`Field`] [`Variant`]s of this enum to consider in code
    /// generation.
    ///
    /// [`Field`]: syn::Field
    /// [`Variant`]: syn::Variant
    pub variants: Vec<syn::Variant>,

    /// Indicator whether this enum has any variants marked with
    /// `#[event(ignore)]` attribute.
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
            .collect::<syn::Result<Vec<_>>>()?;
        if variants.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                "enum must have at least one non-ignored variant",
            ));
        }

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
    /// Validates the given [`syn::Variant`] and parses its [`VariantAttrs`].
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

    /// Substitutes the given [`syn::Generics`] with trivial types.
    ///
    /// - [`syn::Lifetime`] -> `'static`;
    /// - [`syn::Type`] -> `()`.
    fn substitute_generics_trivially(generics: &syn::Generics) -> TokenStream {
        use syn::GenericParam::{Const, Lifetime, Type};

        let generics = generics.params.iter().map(|p| match p {
            Lifetime(_) => quote! { 'static },
            Type(_) => quote! { () },
            Const(c) => quote! { #c },
        });

        quote! { < #( #generics ),* > }
    }

    /// Generates code to derive [`Event`][0] trait, by simply matching over
    /// each enum variant, which is expected to be itself an [`Event`][0]
    /// implementer.
    ///
    /// [0]: arcana_core::es::event::Event
    #[must_use]
    pub fn impl_event(&self) -> TokenStream {
        let ty = &self.ident;
        let (impl_gens, ty_gens, where_clause) = self.generics.split_for_impl();

        let var = self.variants.iter().map(|v| &v.ident).collect::<Vec<_>>();

        let unreachable_arm = self.has_ignored_variants.then(|| {
            quote! { _ => unreachable!(), }
        });

        quote! {
            #[automatically_derived]
            impl #impl_gens ::arcana::es::Event for #ty#ty_gens #where_clause {
                fn name(&self) -> ::arcana::es::event::Name {
                    match self {
                        #(
                            Self::#var(f) => ::arcana::es::Event::name(
                                ::arcana::es::event::BorrowInitial::borrow(f)
                            ),
                        )*
                        #unreachable_arm
                    }
                }

                fn version(&self) -> ::arcana::es::event::Version {
                    match self {
                        #(
                            Self::#var(f) => ::arcana::es::Event::version(
                                ::arcana::es::event::BorrowInitial::borrow(f)
                            ),
                        )*
                        #unreachable_arm
                    }
                }
            }
        }
    }

    /// Generates hidden machinery code used to statically check that all the
    /// [`Event::name`][0]s and [`Event::version`][1]s pairs are corresponding
    /// to single Rust type.
    ///
    /// # Panics
    ///
    /// If some enum [`Variant`]s don't have exactly 1 [`Field`] and not marked
    /// with `#[event(skip)]`. Checked by [`TryFrom`] impl for [`Definition`].
    ///
    /// [0]: arcana_core::es::event::Event::name()
    /// [1]: arcana_core::es::event::Event::version()
    /// [`Field`]: syn::Field
    /// [`Variant`]: syn::Variant
    #[must_use]
    pub fn gen_uniqueness_glue_code(&self) -> TokenStream {
        let ty = &self.ident;
        let (impl_gens, ty_gens, where_clause) = self.generics.split_for_impl();

        let var_ty = self
            .variants
            .iter()
            .flat_map(|v| &v.fields)
            .map(|f| &f.ty)
            .collect::<Vec<_>>();

        // TODO: Use `Self::__arcana_events()` inside impl instead of type
        //       params substitution, once rust-lang/rust#57775 is resolved:
        //       https://github.com/rust-lang/rust/issues/57775
        let ty_subst_gens = Self::substitute_generics_trivially(&self.generics);

        #[rustfmt::skip]
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
            impl #ty#ty_gens {
                #[doc(hidden)]
                pub const fn __arcana_events() -> [
                    (&'static str, &'static str, u16);
                    <Self as ::arcana::codegen::UniqueEvents>::COUNT
                ] {
                    let mut res = [
                        ("", "", 0);
                        <Self as ::arcana::codegen::UniqueEvents>::COUNT
                    ];

                    let mut i = 0;
                    #({
                        let events = <<#var_ty as
                                ::arcana::es::event::UnpackInitial>::Event
                            >::__arcana_events();
                        let mut j = 0;
                        while j < events.len() {
                            res[i] = events[j];
                            j += 1;
                            i += 1;
                        }
                    })*

                    res
                }
            }

            ::arcana::codegen::
    every_combination_of_event_name_and_version_must_correspond_to_single_type!(
                !::arcana::codegen::unique_events::
                    has_different_types_with_same_name_and_ver(
                        #ty::#ty_subst_gens::__arcana_events()
                    )
            );
        }
    }
}

#[cfg(test)]
mod spec {
    use quote::quote;
    use syn::parse_quote;

    #[test]
    fn derives_enum_impl() {
        let input = parse_quote! {
            enum Event {
                File(FileEvent),
                Chat(ChatEvent),
            }
        };

        #[rustfmt::skip]
        let output = quote! {
            #[automatically_derived]
            impl ::arcana::es::Event for Event {
                fn name(&self) -> ::arcana::es::event::Name {
                    match self {
                        Self::File(f) => ::arcana::es::Event::name(
                            ::arcana::es::event::BorrowInitial::borrow(f)
                        ),
                        Self::Chat(f) => ::arcana::es::Event::name(
                            ::arcana::es::event::BorrowInitial::borrow(f)
                        ),
                    }
                }

                fn version(&self) -> ::arcana::es::event::Version {
                    match self {
                        Self::File(f) => ::arcana::es::Event::version(
                            ::arcana::es::event::BorrowInitial::borrow(f)
                        ),
                        Self::Chat(f) => ::arcana::es::Event::version(
                            ::arcana::es::event::BorrowInitial::borrow(f)
                        ),
                    }
                }
            }

            #[automatically_derived]
            #[doc(hidden)]
            impl ::arcana::codegen::UniqueEvents for Event {
                #[doc(hidden)]
                const COUNT: usize =
                    <FileEvent as ::arcana::codegen::UniqueEvents>::COUNT +
                    <ChatEvent as ::arcana::codegen::UniqueEvents>::COUNT;
            }

            #[automatically_derived]
            #[doc(hidden)]
            impl Event {
                #[doc(hidden)]
                pub const fn __arcana_events() -> [
                    (&'static str, &'static str, u16);
                    <Self as ::arcana::codegen::UniqueEvents>::COUNT
                ] {
                    let mut res = [
                        ("", "", 0);
                        <Self as ::arcana::codegen::UniqueEvents>::COUNT
                    ];

                    let mut i = 0;
                    {
                        let events = <<FileEvent as
                                ::arcana::es::event::UnpackInitial>::Event
                            >::__arcana_events();
                        let mut j = 0;
                        while j < events.len() {
                            res[i] = events[j];
                            j += 1;
                            i += 1;
                        }
                    }
                    {
                        let events = <<ChatEvent as
                                ::arcana::es::event::UnpackInitial>::Event
                            >::__arcana_events();
                        let mut j = 0;
                        while j < events.len() {
                            res[i] = events[j];
                            j += 1;
                            i += 1;
                        }
                    }

                    res
                }
            }

            ::arcana::codegen::
    every_combination_of_event_name_and_version_must_correspond_to_single_type!(
                !::arcana::codegen::unique_events::
                    has_different_types_with_same_name_and_ver(
                        Event::<>::__arcana_events()
                    )
            );
        };

        assert_eq!(
            super::derive(input).unwrap().to_string(),
            output.to_string(),
        );
    }

    #[test]
    fn derives_enum_with_generics_impl() {
        let input = parse_quote! {
            enum Event<'a, F, C> {
                File(FileEvent<'a, F>),
                Chat(ChatEvent<'a, C>),
            }
        };

        #[rustfmt::skip]
        let output = quote! {
            #[automatically_derived]
            impl<'a, F, C> ::arcana::es::Event for Event<'a, F, C> {
                fn name(&self) -> ::arcana::es::event::Name {
                    match self {
                        Self::File(f) => ::arcana::es::Event::name(
                            ::arcana::es::event::BorrowInitial::borrow(f)
                        ),
                        Self::Chat(f) => ::arcana::es::Event::name(
                            ::arcana::es::event::BorrowInitial::borrow(f)
                        ),
                    }
                }

                fn version(&self) -> ::arcana::es::event::Version {
                    match self {
                        Self::File(f) => ::arcana::es::Event::version(
                            ::arcana::es::event::BorrowInitial::borrow(f)
                        ),
                        Self::Chat(f) => ::arcana::es::Event::version(
                            ::arcana::es::event::BorrowInitial::borrow(f)
                        ),
                    }
                }
            }

            #[automatically_derived]
            #[doc(hidden)]
            impl<'a, F, C> ::arcana::codegen::UniqueEvents for Event<'a, F, C> {
                #[doc(hidden)]
                const COUNT: usize =
                    <FileEvent<'a, F> as ::arcana::codegen::UniqueEvents>
                        ::COUNT +
                    <ChatEvent<'a, C> as ::arcana::codegen::UniqueEvents>
                        ::COUNT;
            }

            #[automatically_derived]
            #[doc(hidden)]
            impl Event<'a, F, C> {
                #[doc(hidden)]
                pub const fn __arcana_events() -> [
                    (&'static str, &'static str, u16);
                    <Self as ::arcana::codegen::UniqueEvents>::COUNT
                ] {
                    let mut res = [
                        ("", "", 0);
                        <Self as ::arcana::codegen::UniqueEvents>::COUNT
                    ];

                    let mut i = 0;
                    {
                        let events = << FileEvent<'a, F> as
                                ::arcana::es::event::UnpackInitial>::Event
                            >::__arcana_events();
                        let mut j = 0;
                        while j < events.len() {
                            res[i] = events[j];
                            j += 1;
                            i += 1;
                        }
                    }
                    {
                        let events = << ChatEvent<'a, C> as
                                ::arcana::es::event::UnpackInitial>::Event
                            >::__arcana_events();
                        let mut j = 0;
                        while j < events.len() {
                            res[i] = events[j];
                            j += 1;
                            i += 1;
                        }
                    }

                    res
                }
            }

            ::arcana::codegen::
    every_combination_of_event_name_and_version_must_correspond_to_single_type!(
                !::arcana::codegen::unique_events::
                    has_different_types_with_same_name_and_ver(
                        Event::<'static, (), ()>::__arcana_events()
                    )
            );
        };

        assert_eq!(
            super::derive(input).unwrap().to_string(),
            output.to_string(),
        );
    }

    #[allow(clippy::too_many_lines)]
    #[test]
    fn ignores_ignored_variant() {
        let input_ignore = parse_quote! {
            enum Event {
                File(FileEvent),
                Chat(ChatEvent),
                #[event(ignore)]
                _NonExhaustive,
            }
        };
        let input_skip = parse_quote! {
            enum Event {
                File(FileEvent),
                Chat(ChatEvent),
                #[event(skip)]
                _NonExhaustive,
            }
        };

        #[rustfmt::skip]
        let output = quote! {
            #[automatically_derived]
            impl ::arcana::es::Event for Event {
                fn name(&self) -> ::arcana::es::event::Name {
                    match self {
                        Self::File(f) => ::arcana::es::Event::name(
                            ::arcana::es::event::BorrowInitial::borrow(f)
                        ),
                        Self::Chat(f) => ::arcana::es::Event::name(
                            ::arcana::es::event::BorrowInitial::borrow(f)
                        ),
                        _ => unreachable!(),
                    }
                }

                fn version(&self) -> ::arcana::es::event::Version {
                    match self {
                        Self::File(f) => ::arcana::es::Event::version(
                            ::arcana::es::event::BorrowInitial::borrow(f)
                        ),
                        Self::Chat(f) => ::arcana::es::Event::version(
                            ::arcana::es::event::BorrowInitial::borrow(f)
                        ),
                        _ => unreachable!(),
                    }
                }
            }

            #[automatically_derived]
            #[doc(hidden)]
            impl ::arcana::codegen::UniqueEvents for Event {
                #[doc(hidden)]
                const COUNT: usize =
                    <FileEvent as ::arcana::codegen::UniqueEvents>::COUNT +
                    <ChatEvent as ::arcana::codegen::UniqueEvents>::COUNT;
            }

            #[automatically_derived]
            #[doc(hidden)]
            impl Event {
                #[doc(hidden)]
                pub const fn __arcana_events() -> [
                    (&'static str, &'static str, u16);
                    <Self as ::arcana::codegen::UniqueEvents>::COUNT
                ] {
                    let mut res = [
                        ("", "", 0);
                        <Self as ::arcana::codegen::UniqueEvents>::COUNT
                    ];

                    let mut i = 0;
                    {
                        let events = << FileEvent as
                                ::arcana::es::event::UnpackInitial>::Event
                            >::__arcana_events();
                        let mut j = 0;
                        while j < events.len() {
                            res[i] = events[j];
                            j += 1;
                            i += 1;
                        }
                    }
                    {
                        let events = << ChatEvent as
                                ::arcana::es::event::UnpackInitial>::Event
                            >::__arcana_events();
                        let mut j = 0;
                        while j < events.len() {
                            res[i] = events[j];
                            j += 1;
                            i += 1;
                        }
                    }

                    res
                }
            }

            ::arcana::codegen::
    every_combination_of_event_name_and_version_must_correspond_to_single_type!(
                !::arcana::codegen::unique_events::
                    has_different_types_with_same_name_and_ver(
                        Event::<>::__arcana_events()
                    )
            );
        };

        let input_ignore = super::derive(input_ignore).unwrap().to_string();
        let input_skip = super::derive(input_skip).unwrap().to_string();

        assert_eq!(input_ignore, output.to_string());
        assert_eq!(input_skip, input_ignore);
    }

    #[test]
    fn errors_on_multiple_fields_in_variant() {
        let input = parse_quote! {
            enum Event {
                Event1(Event1),
                Event2 {
                    event: Event2,
                    second_field: Event3,
                }
            }
        };

        let err = super::derive(input).unwrap_err();

        assert_eq!(err.to_string(), "enum variants must have exactly 1 field");
    }

    #[test]
    fn errors_on_struct() {
        let input = parse_quote! {
            struct Event;
        };

        let err = super::derive(input).unwrap_err();

        assert_eq!(
            err.to_string(),
            "expected enum only, \
             consider using `arcana::es::event::Versioned` for structs",
        );
    }

    #[test]
    fn errors_on_empty_enum() {
        let input = parse_quote! {
            enum Event {}
        };

        let err = super::derive(input).unwrap_err();

        assert_eq!(
            err.to_string(),
            "enum must have at least one non-ignored variant",
        );
    }

    #[test]
    fn errors_on_enum_with_ignored_variant_only() {
        let input = parse_quote! {
            enum Event {
                #[event(ignore)]
                _NonExhaustive,
            }
        };

        let err = super::derive(input).unwrap_err();

        assert_eq!(
            err.to_string(),
            "enum must have at least one non-ignored variant",
        );
    }
}
