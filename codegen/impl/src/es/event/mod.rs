//! `#[derive(Event)]` macro implementation.

pub mod versioned;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_quote, spanned::Spanned as _};
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
    /// Indicator whether this enum variant should be used as
    /// [`event::Initialized`] rather than [`event::Sourced`].
    ///
    /// [`event::Initialized`]: arcane_core::es::event::Initialized
    /// [`event::Sourced`]: arcane_core::es::event::Sourced
    #[parse(ident, alias = initial)]
    pub init: Option<syn::Ident>,

    /// Indicator whether to ignore this enum variant for code generation.
    #[parse(ident, alias = skip)]
    pub ignore: Option<syn::Ident>,
}

/// Representation of an enum implementing [`Event`], used for code generation.
///
/// [`Event`]: arcane_core::es::event::Event
#[derive(Debug, ToTokens)]
#[to_tokens(append(impl_event, impl_event_sourced, gen_uniqueness_glue_code))]
pub struct Definition {
    /// [`syn::Ident`](struct@syn::Ident) of this enum's type.
    pub ident: syn::Ident,

    /// [`syn::Generics`] of this enum's type.
    pub generics: syn::Generics,

    /// Single-[`Field`] [`Variant`]s of this enum to consider in code
    /// generation, along with the indicator whether this variant should use
    /// [`event::Initialized`] rather than [`event::Sourced`].
    ///
    /// [`event::Initialized`]: arcane_core::es::event::Initialized
    /// [`event::Sourced`]: arcane_core::es::event::Sourced
    /// [`Field`]: syn::Field
    /// [`Variant`]: syn::Variant
    pub variants: Vec<(syn::Variant, bool)>,

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
                 consider using `arcane::es::event::Versioned` for structs",
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
    /// - If [`VariantAttrs::init`] and [`VariantAttrs::ignore`] were specified
    ///   simultaneously.
    /// - If [`syn::Variant`] doesn't have exactly one unnamed 1 [`syn::Field`]
    ///   and is not ignored.
    fn parse_variant(
        variant: &syn::Variant,
    ) -> syn::Result<Option<(syn::Variant, bool)>> {
        let attrs = VariantAttrs::parse_attrs("event", variant)?;

        if let Some(init) = &attrs.init {
            if attrs.ignore.is_some() {
                return Err(syn::Error::new(
                    init.span(),
                    "`init` and `ignore`/`skip` arguments are mutually \
                     exclusive",
                ));
            }
        }

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

        Ok(Some((variant.clone(), attrs.init.is_some())))
    }

    /// Substitutes the given [`syn::Generics`] with trivial types.
    ///
    /// - [`syn::Lifetime`] -> `'static`;
    /// - [`syn::Type`] -> `()`.
    ///
    /// [`syn::Lifetime`]: struct@syn::Lifetime
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
    /// [0]: arcane_core::es::event::Event
    #[must_use]
    pub fn impl_event(&self) -> TokenStream {
        let ty = &self.ident;
        let (impl_gens, ty_gens, where_clause) = self.generics.split_for_impl();

        let var = self.variants.iter().map(|v| &v.0.ident).collect::<Vec<_>>();

        let unreachable_arm = self.has_ignored_variants.then(|| {
            quote! { _ => unreachable!(), }
        });

        quote! {
            #[automatically_derived]
            impl #impl_gens ::arcane::es::Event for #ty #ty_gens #where_clause {
                fn name(&self) -> ::arcane::es::event::Name {
                    match self {
                        #(
                            Self::#var(f) => ::arcane::es::Event::name(f),
                        )*
                        #unreachable_arm
                    }
                }

                fn version(&self) -> ::arcane::es::event::Version {
                    match self {
                        #(
                            Self::#var(f) => ::arcane::es::Event::version(f),
                        )*
                        #unreachable_arm
                    }
                }
            }
        }
    }

    /// Generates code to derive [`event::Sourced`][0] trait, by simply matching
    /// each enum variant, which is expected to have itself an
    /// [`event::Sourced`][0] implementation.
    ///
    /// [0]: arcane_core::es::event::Sourced
    #[must_use]
    pub fn impl_event_sourced(&self) -> TokenStream {
        let ty = &self.ident;
        let (_, ty_gens, _) = self.generics.split_for_impl();
        let turbofish_gens = ty_gens.as_turbofish();

        let var_tys = self.variants.iter().map(|(v, is_initial)| {
            let var_ty = v.fields.iter().next().map(|f| &f.ty);
            if *is_initial {
                quote! { ::arcane::es::event::Initial<#var_ty> }
            } else {
                quote! { #var_ty }
            }
        });

        let mut ext_gens = self.generics.clone();
        ext_gens.params.push(parse_quote! { __S });
        ext_gens.make_where_clause().predicates.push(parse_quote! {
            Self: #( ::arcane::es::event::Sourced<#var_tys> )+*
        });
        let (impl_gens, _, where_clause) = ext_gens.split_for_impl();

        let arms = self.variants.iter().map(|(v, is_initial)| {
            let var = &v.ident;
            let var_ty = v.fields.iter().next().map(|f| &f.ty);

            let event = if *is_initial {
                quote! {
                    <::arcane::es::event::Initial<#var_ty>
                     as ::arcane::RefCast>::ref_cast(f)
                }
            } else {
                quote! { f }
            };
            quote! {
                #ty #turbofish_gens::#var(f) => {
                    ::arcane::es::event::Sourced::apply(self, #event);
                },
            }
        });
        let unreachable_arm = self.has_ignored_variants.then(|| {
            quote! { _ => unreachable!(), }
        });

        quote! {
            #[automatically_derived]
            impl #impl_gens ::arcane::es::event::Sourced<#ty #ty_gens>
                for Option<__S> #where_clause
            {
                fn apply(&mut self, event: &#ty #ty_gens) {
                    match event {
                        #( #arms )*
                        #unreachable_arm
                    }
                }
            }
        }
    }

    /// Generates hidden machinery code used to statically check that all the
    /// [`Event::name`][0]s and [`Event::version`][1]s pairs are corresponding
    /// to a single Rust type.
    ///
    /// # Panics
    ///
    /// If some enum [`Variant`]s don't have exactly 1 [`Field`] and not marked
    /// with `#[event(skip)]`. Checked by [`TryFrom`] impl for [`Definition`].
    ///
    /// [0]: arcane_core::es::event::Event::name()
    /// [1]: arcane_core::es::event::Event::version()
    /// [`Field`]: syn::Field
    /// [`Variant`]: syn::Variant
    #[must_use]
    pub fn gen_uniqueness_glue_code(&self) -> TokenStream {
        let ty = &self.ident;
        let (impl_gens, ty_gens, where_clause) = self.generics.split_for_impl();

        let var_ty = self
            .variants
            .iter()
            .flat_map(|v| &v.0.fields)
            .map(|f| &f.ty)
            .collect::<Vec<_>>();

        // TODO: Use `has_different_types_with_same_name_and_ver` inside impl
        //       instead of type params substitution, once rust-lang/rust#57775
        //       is resolved: https://github.com/rust-lang/rust/issues/57775
        let ty_subst_gens = Self::substitute_generics_trivially(&self.generics);
        let const_phantom_generics =
            Self::gen_const_phantom_generics(&self.generics);

        let glue = quote! { ::arcane::es::event::codegen };
        quote! {
            #[automatically_derived]
            #[allow(unsafe_code)]
            #[doc(hidden)]
            unsafe impl #impl_gens #glue ::Events for #ty #ty_gens
                #where_clause
            {
                #[doc(hidden)]
                const EVENTS: &'static [(&'static str, &'static str, u16)] = {
                    #const_phantom_generics
                    #glue ::const_concat_slices!(
                        (&'static str, &'static str, u16),
                        #( <#var_ty as #glue ::Events>::EVENTS ),*
                    )
                };
            }

            #[automatically_derived]
            #[doc(hidden)]
            const _: () = ::std::assert!(
                !#glue ::has_different_types_with_same_name_and_ver::<
                    #ty #ty_subst_gens
                >(),
                "having different `Event` types with the same name and version \
                 inside a single enum is forbidden",
            );
        }
    }

    /// Replaces all type parameters with `()` type. This required for const
    /// contexts, where type parameters are not allowed.
    // TODO: Remove this, once rust-lang/rust#57775 is resolved:
    //       https://github.com/rust-lang/rust/issues/57775
    fn gen_const_phantom_generics(generics: &syn::Generics) -> TokenStream {
        let ty = generics.type_params().map(|p| {
            let ident = &p.ident;
            quote! { type #ident = (); }
        });

        quote! { #( #ty )* }
    }
}

#[cfg(test)]
mod spec {
    use quote::quote;
    use syn::parse_quote;

    #[allow(clippy::too_many_lines)]
    #[test]
    fn derives_enum_impl() {
        let input = parse_quote! {
            enum Event {
                #[event(init)]
                File(FileEvent),
                Chat(ChatEvent),
            }
        };

        let output = quote! {
            #[automatically_derived]
            impl ::arcane::es::Event for Event {
                fn name(&self) -> ::arcane::es::event::Name {
                    match self {
                        Self::File(f) => ::arcane::es::Event::name(f),
                        Self::Chat(f) => ::arcane::es::Event::name(f),
                    }
                }

                fn version(&self) -> ::arcane::es::event::Version {
                    match self {
                        Self::File(f) => ::arcane::es::Event::version(f),
                        Self::Chat(f) => ::arcane::es::Event::version(f),
                    }
                }
            }

            #[automatically_derived]
            impl<__S> ::arcane::es::event::Sourced<Event> for Option<__S>
            where
                Self: ::arcane::es::event::Sourced<
                          ::arcane::es::event::Initial<FileEvent>
                      > +
                      ::arcane::es::event::Sourced<ChatEvent>
            {
                fn apply(&mut self, event: &Event) {
                    match event {
                        Event::File(f) => {
                            ::arcane::es::event::Sourced::apply(
                                self,
                                <::arcane::es::event::Initial<FileEvent>
                                 as ::arcane::RefCast>::ref_cast(f)
                            );
                        },
                        Event::Chat(f) => {
                            ::arcane::es::event::Sourced::apply(self, f);
                        },
                    }
                }
            }

            #[automatically_derived]
            #[allow(unsafe_code)]
            #[doc(hidden)]
            unsafe impl ::arcane::es::event::codegen::Events for Event {
                #[doc(hidden)]
                const EVENTS: &'static [(&'static str, &'static str, u16)] = {
                    ::arcane::es::event::codegen::const_concat_slices!(
                        (&'static str, &'static str, u16),
                        <FileEvent
                            as ::arcane::es::event::codegen::Events>::EVENTS,
                        <ChatEvent
                            as ::arcane::es::event::codegen::Events>::EVENTS
                    )
                };
            }

            #[automatically_derived]
            #[doc(hidden)]
            const _: () = ::std::assert!(
                !::arcane::es::event::codegen::
                    has_different_types_with_same_name_and_ver::< Event<> >(),
                "having different `Event` types with the same name and version \
                 inside a single enum is forbidden",
            );
        };

        assert_eq!(
            super::derive(input).unwrap().to_string(),
            output.to_string(),
        );
    }

    #[allow(clippy::too_many_lines)]
    #[test]
    fn derives_enum_with_generics_impl() {
        let input = parse_quote! {
            enum Event<'a, F, C> {
                #[event(init)]
                File(FileEvent<'a, F>),
                Chat(ChatEvent<'a, C>),
            }
        };

        let output = quote! {
            #[automatically_derived]
            impl<'a, F, C> ::arcane::es::Event for Event<'a, F, C> {
                fn name(&self) -> ::arcane::es::event::Name {
                    match self {
                        Self::File(f) => ::arcane::es::Event::name(f),
                        Self::Chat(f) => ::arcane::es::Event::name(f),
                    }
                }

                fn version(&self) -> ::arcane::es::event::Version {
                    match self {
                        Self::File(f) => ::arcane::es::Event::version(f),
                        Self::Chat(f) => ::arcane::es::Event::version(f),
                    }
                }
            }

            #[automatically_derived]
            impl<'a, F, C, __S> ::arcane::es::event::Sourced<Event<'a, F, C> >
                for Option<__S>
            where
                Self: ::arcane::es::event::Sourced<
                          ::arcane::es::event::Initial<FileEvent<'a, F> >
                      > +
                      ::arcane::es::event::Sourced<ChatEvent<'a, C> >
            {
                fn apply(&mut self, event: &Event<'a, F, C>) {
                    match event {
                        Event::<'a, F, C>::File(f) => {
                            ::arcane::es::event::Sourced::apply(
                                self,
                                <::arcane::es::event::Initial<FileEvent<'a, F> >
                                 as ::arcane::RefCast>::ref_cast(f)
                            );
                        },
                        Event::<'a, F, C>::Chat(f) => {
                            ::arcane::es::event::Sourced::apply(self, f);
                        },
                    }
                }
            }

            #[automatically_derived]
            #[allow(unsafe_code)]
            #[doc(hidden)]
            unsafe impl<'a, F, C> ::arcane::es::event::codegen::Events
                for Event<'a, F, C>
            {
                #[doc(hidden)]
                const EVENTS: &'static [(&'static str, &'static str, u16)] = {
                    type F = ();
                    type C = ();

                    ::arcane::es::event::codegen::const_concat_slices!(
                        (&'static str, &'static str, u16),
                        <FileEvent<'a, F>
                            as ::arcane::es::event::codegen::Events>::EVENTS,
                        <ChatEvent<'a, C>
                            as ::arcane::es::event::codegen::Events>::EVENTS
                    )
                };
            }

            #[automatically_derived]
            #[doc(hidden)]
            const _: () = ::std::assert!(
                !::arcane::es::event::codegen::
                    has_different_types_with_same_name_and_ver::<
                        Event<'static, (), ()>
                    >(),
                "having different `Event` types with the same name and version \
                 inside a single enum is forbidden",
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

        let output = quote! {
            #[automatically_derived]
            impl ::arcane::es::Event for Event {
                fn name(&self) -> ::arcane::es::event::Name {
                    match self {
                        Self::File(f) => ::arcane::es::Event::name(f),
                        Self::Chat(f) => ::arcane::es::Event::name(f),
                        _ => unreachable!(),
                    }
                }

                fn version(&self) -> ::arcane::es::event::Version {
                    match self {
                        Self::File(f) => ::arcane::es::Event::version(f),
                        Self::Chat(f) => ::arcane::es::Event::version(f),
                        _ => unreachable!(),
                    }
                }
            }

            #[automatically_derived]
            impl<__S> ::arcane::es::event::Sourced<Event> for Option<__S>
            where
                Self: ::arcane::es::event::Sourced<FileEvent> +
                      ::arcane::es::event::Sourced<ChatEvent>
            {
                fn apply(&mut self, event: &Event) {
                    match event {
                        Event::File(f) => {
                            ::arcane::es::event::Sourced::apply(self, f);
                        },
                        Event::Chat(f) => {
                            ::arcane::es::event::Sourced::apply(self, f);
                        },
                        _ => unreachable!(),
                    }
                }
            }

            #[automatically_derived]
            #[allow(unsafe_code)]
            #[doc(hidden)]
            unsafe impl ::arcane::es::event::codegen::Events for Event {
                #[doc(hidden)]
                const EVENTS: &'static [(&'static str, &'static str, u16)] = {
                    ::arcane::es::event::codegen::const_concat_slices!(
                        (&'static str, &'static str, u16),
                        <FileEvent
                            as ::arcane::es::event::codegen::Events>::EVENTS,
                        <ChatEvent
                            as ::arcane::es::event::codegen::Events>::EVENTS
                    )
                };
            }

            #[automatically_derived]
            #[doc(hidden)]
            const _: () = ::std::assert!(
                !::arcane::es::event::codegen::
                    has_different_types_with_same_name_and_ver::< Event<> >(),
                "having different `Event` types with the same name and version \
                 inside a single enum is forbidden",
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
             consider using `arcane::es::event::Versioned` for structs",
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

    #[test]
    fn errors_on_both_init_and_ignored_variant() {
        let input = parse_quote! {
            enum Event {
                #[event(init, ignore)]
                Event1(Event1),
            }
        };

        let err = super::derive(input).unwrap_err();

        assert_eq!(
            err.to_string(),
            "`init` and `ignore`/`skip` arguments are mutually exclusive",
        );
    }
}
