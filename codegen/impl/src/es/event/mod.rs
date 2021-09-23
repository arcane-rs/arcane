//! `#[derive(Event)]` macro implementation.

pub mod transformer;
pub mod versioned;

use std::{convert::TryFrom, iter};

use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_quote, punctuated::Punctuated, spanned::Spanned as _, Token};
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
#[to_tokens(append(
    impl_event,
    impl_event_sourced,
    gen_uniqueness_glue_code,
    impl_transformer,
))]
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

    /// TODO
    pub fn variants_types(&self) -> impl Iterator<Item = &syn::Field> {
        self.variants.iter().flat_map(|v| &v.fields)
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
                                ::arcana::es::event::codegen::Borrow::borrow(f),
                            ),
                        )*
                        #unreachable_arm
                    }
                }

                fn version(&self) -> ::arcana::es::event::Version {
                    match self {
                        #(
                            Self::#var(f) => ::arcana::es::Event::version(
                                ::arcana::es::event::codegen::Borrow::borrow(f),
                            ),
                        )*
                        #unreachable_arm
                    }
                }
            }
        }
    }

    /// Generates code to derive [`event::Sourced`][0] trait, by simply matching
    /// each enum variant, which is expected to have itself
    /// [`event::Sourced`][0] implementation.
    ///
    /// [0]: arcana_core::es::event::Sourced
    #[must_use]
    pub fn impl_event_sourced(&self) -> TokenStream {
        let ty = &self.ident;
        let (_, ty_gens, _) = self.generics.split_for_impl();
        let turbofish_gens = ty_gens.as_turbofish();

        let var_ty = self.variants_types();

        let mut ext_gens = self.generics.clone();
        ext_gens.params.push(parse_quote! { __S });
        ext_gens.make_where_clause().predicates.push(parse_quote! {
            Self: #( ::arcana::es::event::Sourced<#var_ty> )+*
        });
        let (impl_gens, _, where_clause) = ext_gens.split_for_impl();

        let var = self.variants.iter().map(|v| &v.ident);

        let unreachable_arm = self.has_ignored_variants.then(|| {
            quote! { _ => unreachable!(), }
        });

        quote! {
            #[automatically_derived]
            impl #impl_gens ::arcana::es::event::Sourced<#ty#ty_gens>
                for Option<__S> #where_clause
            {
                fn apply(&mut self, event: &#ty#ty_gens) {
                    match event {
                        #(#ty#turbofish_gens::#var(f) => {
                            ::arcana::es::event::Sourced::apply(self, f)
                        },)*
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
    /// [0]: arcana_core::es::event::Event::name()
    /// [1]: arcana_core::es::event::Event::version()
    /// [`Field`]: syn::Field
    /// [`Variant`]: syn::Variant
    #[must_use]
    pub fn gen_uniqueness_glue_code(&self) -> TokenStream {
        let ty = &self.ident;
        let (impl_gens, ty_gens, where_clause) = self.generics.split_for_impl();

        let var_ty = self.variants_types().collect::<Vec<_>>();

        // TODO: Use `Self::__arcana_events()` inside impl instead of type
        //       params substitution, once rust-lang/rust#57775 is resolved:
        //       https://github.com/rust-lang/rust/issues/57775
        let ty_subst_gens = Self::substitute_generics_trivially(&self.generics);

        let glue = quote! { ::arcana::es::event::codegen };
        quote! {
            #[automatically_derived]
            #[doc(hidden)]
            impl #impl_gens #glue::Versioned for #ty#ty_gens
                 #where_clause
            {
                #[doc(hidden)]
                const COUNT: usize =
                    #( <#var_ty as #glue::Versioned>::COUNT )+*;
            }

            #[automatically_derived]
            #[doc(hidden)]
            impl #ty#ty_gens {
                #[doc(hidden)]
                pub const fn __arcana_events() -> [
                    (&'static str, &'static str, u16);
                    <Self as #glue::Versioned>::COUNT
                ] {
                    let mut res = [
                        ("", "", 0); <Self as #glue::Versioned>::COUNT
                    ];

                    let mut i = 0;
                    #({
                        let events = <
                            <#var_ty as #glue::Unpacked>::Type
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

            ::arcana::es::event::
           each_combination_of_name_and_version_must_correspond_to_single_type!(
                !#glue::has_different_types_with_same_name_and_ver(
                    #ty::#ty_subst_gens::__arcana_events()
                )
            );
        }
    }

    /// Generates code to derive [`Transformer`][0] trait, by matching over each
    /// enum variant, which is expected to have itself [`Transformer`][0]
    /// implementation.
    ///
    /// [0]: arcana_core::es::adapter::Transformer
    #[must_use]
    pub fn impl_transformer(&self) -> TokenStream {
        let adapter_path = quote! { ::arcana::es::adapter };
        let event = &self.ident;

        let generics = self.transformer_generics();
        let (impl_gen, _, where_clause) = generics.split_for_impl();
        let (_, type_gen, _) = self.generics.split_for_impl();

        let context_bound = self.transformer_context_bound();
        let transformed = self.transformed_stream();
        let inner_match = self.transformer_inner_match();

        quote! {
            #[automatically_derived]
            impl#impl_gen ::arcana::es::adapter::Transformer<#event#type_gen>
                for #adapter_path::Wrapper<__A> #where_clause
            {
                type Context<__Impl> = #context_bound;
                type Error = <__A as #adapter_path::Returning>::Error;
                type Transformed =
                    <__A as #adapter_path::Returning>::Transformed;
                type TransformedStream<'out, __Ctx: 'out> = #transformed;

                fn transform<'me, 'ctx, 'out, __Ctx>(
                    &'me self,
                    __event: #event#type_gen,
                    __context: &'ctx __Ctx,
                ) -> <Self as #adapter_path::Transformer<#event>>::
                    TransformedStream<'out, __Ctx>
                where
                    'me: 'out,
                    'ctx: 'out,
                    __Ctx: 'out,
                {
                    match __event {
                        #inner_match
                    }
                }
            }
        }
    }

    /// Generates code of [`Transformer::Transformed`][0] associated type.
    ///
    /// This is basically a recursive type
    /// [`Either`]`<Var1, `[`Either`]`<Var2, ...>>`, where every `VarN` is an
    /// enum variant's [`Transformer::TransformedStream`][1] wrapped in a
    /// [`stream::Map`] with a function that uses [`From`] impl to transform
    /// [`Event`]s into compatible ones.
    ///
    /// [0]: arcana_core::es::adapter::Transformer::Transformed
    /// [1]: arcana_core::es::adapter::Transformer::TransformedStream
    /// [`Either`]: futures::future::Either
    /// [`Event`]: trait@::arcana_core::es::Event
    /// [`stream::Map`]: futures::stream::Map
    #[must_use]
    pub fn transformed_stream(&self) -> TokenStream {
        let adapter_path = quote! { ::arcana::es::adapter };
        let event = &self.ident;

        let transformed_stream = |from: &syn::Type| {
            quote! {
                #adapter_path::codegen::futures::stream::Map<
                    <Self as #adapter_path::Transformer<#from >>::
                        TransformedStream<'out, __Ctx>,
                    fn(
                        ::std::result::Result<
                            <Self as #adapter_path::Transformer<#from>>::
                                Transformed,
                            <Self as #adapter_path::Transformer<#from>>::
                                Error,
                        >,
                    ) -> ::std::result::Result<
                        <Self as #adapter_path::Transformer<#event>>::
                            Transformed,
                        <Self as #adapter_path::Transformer<#event>>::
                            Error,
                    >
                >
            }
        };

        self.variants_types()
            .fold(None, |acc, field| {
                let variant_stream = transformed_stream(&field.ty);
                Some(
                    acc.map(|acc| {
                        quote! {
                            ::arcana::es::adapter::codegen::futures::future::
                            Either<
                                #variant_stream,
                                #acc,
                            >
                        }
                    })
                    .unwrap_or(variant_stream),
                )
            })
            .unwrap_or_default()
    }

    /// Generates code for implementation of a [`Transformer::transform()`][0]
    /// fn.
    ///
    /// Matches over every [`Event`]'s variant and makes it compatible with
    /// [`Self::transformed_stream()`] type with [`StreamExt::left_stream()`]
    /// and [`StreamExt::right_stream()`] combinators.
    ///
    /// [0]: arcana_core::es::adapter::Transformer::transform
    /// [`Event`]: arcana_core::es::Event
    /// [`StreamExt::left_stream()`]: futures::StreamExt::left_stream()
    /// [`StreamExt::right_stream()`]: futures::StreamExt::right_stream()
    #[must_use]
    pub fn transformer_inner_match(&self) -> TokenStream {
        let adapter_path = quote! { ::arcana::es::adapter };
        let event = &self.ident;

        self.variants
            .iter()
            .filter_map(|var| {
                var.fields.iter().next().map(|f| (&var.ident, &f.ty))
            })
            .enumerate()
            .map(|(i, (variant_ident, var_ty))| {
                let stream_map = quote! {
                    #adapter_path::codegen::futures::StreamExt::map(
                        <Self as #adapter_path::Transformer<#var_ty>>::
                            transform(self, __event, __context),
                        {
                            let __transform_fn: fn(_) -> _ = |__res| {
                                ::std::result::Result::map_err(
                                    ::std::result::Result::map(
                                        __res,
                                        ::std::convert::Into::into,
                                    ),
                                    ::std::convert::Into::into,
                                )
                            };
                            __transform_fn
                        },
                    )
                };

                let right_stream = quote! {
                    #adapter_path::futures::StreamExt::right_stream
                };
                let left_stream = quote! {
                    #adapter_path::codegen::futures::StreamExt::left_stream
                };
                let left_stream_count =
                    (i == self.variants.len() - 1).then(|| 0).unwrap_or(1);

                let transformed_stream = iter::repeat(left_stream)
                    .take(left_stream_count)
                    .chain(iter::repeat(right_stream).take(i))
                    .fold(stream_map, |acc, stream| {
                        quote! { #stream(#acc) }
                    });

                quote! {
                    #event::#variant_ident(__event) => {
                        #transformed_stream
                    },
                }
            })
            .collect()
    }

    /// Generates [`Transformer::Context`][1] type to constrain `Ctx` in
    /// [`transform()`][2] method.
    ///
    /// [1]: arcana_core::es::adapter::Transformer::Context
    /// [2]: arcana_core::es::adapter::Transformer::transform
    #[must_use]
    pub fn transformer_context_bound(&self) -> TokenStream {
        let adapter_path = quote! { ::arcana::es::adapter };
        self.variants_types()
            .map(|f| {
                quote! {
                    <Self as #adapter_path::Transformer<#f>>::Context<__Impl>
                }
            })
            .fold(None, |acc, var_ty| {
                Some(
                    acc.map(|acc| quote! { #adapter_path::And<#var_ty, #acc> })
                        .unwrap_or(var_ty),
                )
            })
            .unwrap_or_default()
    }

    /// Generates [`syn::Generics`] for [`Transformer`] impl.
    ///
    /// 1. Adds `__A` generic parameter for [`Adapter`];
    /// 2. Ensures `__A` implements [`Returning`];
    /// 3. Ensures [`Wrapper`]`<__A>` implements [`Transformer`] for every enum
    /// variant;
    /// 4. Ensures [`Transformed`] and [`Error`] are implementing [`From`] for
    /// every enum variant's [`Transformed`] and [`Error`] and they are all
    /// `'static`.
    ///
    /// [`Adapter`]: arcana_core::es::Adapter
    /// [`Error`]: arcana_core::es::adapter::Transformer::Error
    /// [`Returning`]: arcana_core::es::adapter::Returning
    /// [`Transformed`]: arcana_core::es::adapter::Transformer::Transformed
    /// [`Transformer`]: arcana_core::es::adapter::Transformer
    /// [`Wrapper`]: arcana_core::es::adapter::Wrapper
    #[must_use]
    pub fn transformer_generics(&self) -> syn::Generics {
        let adapter_path = quote! { ::arcana::es::adapter };
        let var_ty = self.variants_types().collect::<Vec<_>>();

        let bounds: Punctuated<syn::WherePredicate, Token![,]> = parse_quote! {
            __A: #adapter_path::Returning,
            Self: #( #adapter_path::Transformer<#var_ty> )+*,
            <__A as #adapter_path::Returning>::Transformed:
                #( ::std::convert::From<
                    <Self as #adapter_path::Transformer<#var_ty>>::Transformed
                > )+*
                + 'static,
            <__A as #adapter_path::Returning>::Error:
                #( ::std::convert::From<
                    <Self as #adapter_path::Transformer<#var_ty>>::Error
                > )+*
                + 'static,
            #(
                <Self as #adapter_path::Transformer<#var_ty>>::Transformed:
                    'static,
                <Self as #adapter_path::Transformer<#var_ty>>::Error:
                    'static,
            )*
        };

        let mut generics = self.generics.clone();
        generics.params.push(parse_quote! { __A });
        generics.make_where_clause().predicates.extend(bounds);

        generics
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
                File(FileEvent),
                Chat(ChatEvent),
            }
        };

        let output = quote! {
            #[automatically_derived]
            impl ::arcana::es::Event for Event {
                fn name(&self) -> ::arcana::es::event::Name {
                    match self {
                        Self::File(f) => ::arcana::es::Event::name(
                            ::arcana::es::event::codegen::Borrow::borrow(f),
                        ),
                        Self::Chat(f) => ::arcana::es::Event::name(
                            ::arcana::es::event::codegen::Borrow::borrow(f),
                        ),
                    }
                }

                fn version(&self) -> ::arcana::es::event::Version {
                    match self {
                        Self::File(f) => ::arcana::es::Event::version(
                            ::arcana::es::event::codegen::Borrow::borrow(f),
                        ),
                        Self::Chat(f) => ::arcana::es::Event::version(
                            ::arcana::es::event::codegen::Borrow::borrow(f),
                        ),
                    }
                }
            }

            #[automatically_derived]
            impl<__S> ::arcana::es::event::Sourced<Event> for Option<__S>
            where
                Self: ::arcana::es::event::Sourced<FileEvent> +
                      ::arcana::es::event::Sourced<ChatEvent>
            {
                fn apply(&mut self, event: &Event) {
                    match event {
                        Event::File(f) => {
                            ::arcana::es::event::Sourced::apply(self, f)
                        },
                        Event::Chat(f) => {
                            ::arcana::es::event::Sourced::apply(self, f)
                        },
                    }
                }
            }

            #[automatically_derived]
            #[doc(hidden)]
            impl ::arcana::es::event::codegen::Versioned for Event {
                #[doc(hidden)]
                const COUNT: usize =
                    <FileEvent
                     as ::arcana::es::event::codegen::Versioned>::COUNT +
                    <ChatEvent
                     as ::arcana::es::event::codegen::Versioned>::COUNT;
            }

            #[automatically_derived]
            #[doc(hidden)]
            impl Event {
                #[doc(hidden)]
                pub const fn __arcana_events() -> [
                    (&'static str, &'static str, u16);
                    <Self as ::arcana::es::event::codegen::Versioned>::COUNT
                ] {
                    let mut res = [
                        ("", "", 0);
                        <Self as ::arcana::es::event::codegen::Versioned>::COUNT
                    ];

                    let mut i = 0;
                    {
                        let events = <
                            <FileEvent
                             as ::arcana::es::event::codegen::Unpacked>::Type
                        >::__arcana_events();
                        let mut j = 0;
                        while j < events.len() {
                            res[i] = events[j];
                            j += 1;
                            i += 1;
                        }
                    }
                    {
                        let events = <
                            <ChatEvent
                             as ::arcana::es::event::codegen::Unpacked>::Type
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

            ::arcana::es::event::
           each_combination_of_name_and_version_must_correspond_to_single_type!(
                !::arcana::es::event::codegen::
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

    #[allow(clippy::too_many_lines)]
    #[test]
    fn derives_enum_with_generics_impl() {
        let input = parse_quote! {
            enum Event<'a, F, C> {
                File(FileEvent<'a, F>),
                Chat(ChatEvent<'a, C>),
            }
        };

        let output = quote! {
            #[automatically_derived]
            impl<'a, F, C> ::arcana::es::Event for Event<'a, F, C> {
                fn name(&self) -> ::arcana::es::event::Name {
                    match self {
                        Self::File(f) => ::arcana::es::Event::name(
                            ::arcana::es::event::codegen::Borrow::borrow(f),
                        ),
                        Self::Chat(f) => ::arcana::es::Event::name(
                            ::arcana::es::event::codegen::Borrow::borrow(f),
                        ),
                    }
                }

                fn version(&self) -> ::arcana::es::event::Version {
                    match self {
                        Self::File(f) => ::arcana::es::Event::version(
                            ::arcana::es::event::codegen::Borrow::borrow(f),
                        ),
                        Self::Chat(f) => ::arcana::es::Event::version(
                            ::arcana::es::event::codegen::Borrow::borrow(f),
                        ),
                    }
                }
            }

            #[automatically_derived]
            impl<'a, F, C, __S> ::arcana::es::event::Sourced<Event<'a, F, C> >
                for Option<__S>
            where
                Self: ::arcana::es::event::Sourced<FileEvent<'a, F> > +
                      ::arcana::es::event::Sourced<ChatEvent<'a, C> >
            {
                fn apply(&mut self, event: &Event<'a, F, C>) {
                    match event {
                        Event::<'a, F, C>::File(f) => {
                            ::arcana::es::event::Sourced::apply(self, f)
                        },
                        Event::<'a, F, C>::Chat(f) => {
                            ::arcana::es::event::Sourced::apply(self, f)
                        },
                    }
                }
            }

            #[automatically_derived]
            #[doc(hidden)]
            impl<'a, F, C> ::arcana::es::event::codegen::Versioned
                for Event<'a, F, C>
            {
                #[doc(hidden)]
                const COUNT: usize =
                    <FileEvent<'a, F>
                     as ::arcana::es::event::codegen::Versioned>::COUNT +
                    <ChatEvent<'a, C>
                     as ::arcana::es::event::codegen::Versioned>::COUNT;
            }

            #[automatically_derived]
            #[doc(hidden)]
            impl Event<'a, F, C> {
                #[doc(hidden)]
                pub const fn __arcana_events() -> [
                    (&'static str, &'static str, u16);
                    <Self as ::arcana::es::event::codegen::Versioned>::COUNT
                ] {
                    let mut res = [
                        ("", "", 0);
                        <Self as ::arcana::es::event::codegen::Versioned>::COUNT
                    ];

                    let mut i = 0;
                    {
                        let events = <
                            <FileEvent<'a, F>
                             as ::arcana::es::event::codegen::Unpacked>::Type
                        >::__arcana_events();
                        let mut j = 0;
                        while j < events.len() {
                            res[i] = events[j];
                            j += 1;
                            i += 1;
                        }
                    }
                    {
                        let events = <
                            <ChatEvent<'a, C>
                             as ::arcana::es::event::codegen::Unpacked>::Type
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

            ::arcana::es::event::
           each_combination_of_name_and_version_must_correspond_to_single_type!(
                !::arcana::es::event::codegen::
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

        let output = quote! {
            #[automatically_derived]
            impl ::arcana::es::Event for Event {
                fn name(&self) -> ::arcana::es::event::Name {
                    match self {
                        Self::File(f) => ::arcana::es::Event::name(
                            ::arcana::es::event::codegen::Borrow::borrow(f),
                        ),
                        Self::Chat(f) => ::arcana::es::Event::name(
                            ::arcana::es::event::codegen::Borrow::borrow(f),
                        ),
                        _ => unreachable!(),
                    }
                }

                fn version(&self) -> ::arcana::es::event::Version {
                    match self {
                        Self::File(f) => ::arcana::es::Event::version(
                            ::arcana::es::event::codegen::Borrow::borrow(f),
                        ),
                        Self::Chat(f) => ::arcana::es::Event::version(
                            ::arcana::es::event::codegen::Borrow::borrow(f),
                        ),
                        _ => unreachable!(),
                    }
                }
            }

            #[automatically_derived]
            impl<__S> ::arcana::es::event::Sourced<Event> for Option<__S>
            where
                Self: ::arcana::es::event::Sourced<FileEvent> +
                      ::arcana::es::event::Sourced<ChatEvent>
            {
                fn apply(&mut self, event: &Event) {
                    match event {
                        Event::File(f) => {
                            ::arcana::es::event::Sourced::apply(self, f)
                        },
                        Event::Chat(f) => {
                            ::arcana::es::event::Sourced::apply(self, f)
                        },
                        _ => unreachable!(),
                    }
                }
            }

            #[automatically_derived]
            #[doc(hidden)]
            impl ::arcana::es::event::codegen::Versioned for Event {
                #[doc(hidden)]
                const COUNT: usize =
                    <FileEvent
                     as ::arcana::es::event::codegen::Versioned>::COUNT +
                    <ChatEvent
                     as ::arcana::es::event::codegen::Versioned>::COUNT;
            }

            #[automatically_derived]
            #[doc(hidden)]
            impl Event {
                #[doc(hidden)]
                pub const fn __arcana_events() -> [
                    (&'static str, &'static str, u16);
                    <Self as ::arcana::es::event::codegen::Versioned>::COUNT
                ] {
                    let mut res = [
                        ("", "", 0);
                        <Self as ::arcana::es::event::codegen::Versioned>::COUNT
                    ];

                    let mut i = 0;
                    {
                        let events = <
                            <FileEvent
                             as ::arcana::es::event::codegen::Unpacked>::Type
                        >::__arcana_events();
                        let mut j = 0;
                        while j < events.len() {
                            res[i] = events[j];
                            j += 1;
                            i += 1;
                        }
                    }
                    {
                        let events = <
                            <ChatEvent
                             as ::arcana::es::event::codegen::Unpacked>::Type
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

            ::arcana::es::event::
           each_combination_of_name_and_version_must_correspond_to_single_type!(
                !::arcana::es::event::codegen::
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
