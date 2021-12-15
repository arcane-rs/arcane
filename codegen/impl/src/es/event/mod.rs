//! `#[derive(Event)]` macro implementation.

pub mod adapter;
pub mod versioned;

use std::{convert::TryFrom, iter};

use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_quote, punctuated::Punctuated, spanned::Spanned as _};
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
    /// [`event::Initialized`]: arcana_core::es::event::Initialized
    /// [`event::Sourced`]: arcana_core::es::event::Sourced
    #[parse(ident, alias = initial)]
    pub init: Option<syn::Ident>,

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
    impl_transformer,
    gen_uniqueness_glue_code,
))]
pub struct Definition {
    /// [`syn::Ident`](struct@syn::Ident) of this enum's type.
    pub ident: syn::Ident,

    /// [`syn::Generics`] of this enum's type.
    pub generics: syn::Generics,

    /// Single-[`Field`] [`Variant`]s of this enum to consider in code
    /// generation, along with the indicator whether this variant should use
    /// [`event::Initialized`] rather than [`event::Sourced`].
    ///
    /// [`event::Initialized`]: arcana_core::es::event::Initialized
    /// [`event::Sourced`]: arcana_core::es::event::Sourced
    /// [`Field`]: syn::Field
    /// [`Variant`]: syn::Variant
    pub variants: Vec<SingleFieldVariant>,

    /// Indicator whether this enum has any variants marked with
    /// `#[event(ignore)]` attribute.
    pub has_ignored_variants: bool,
}

/// Parsed single-field enum variant for `#[derive(Event)]` macro.
#[derive(Clone, Debug)]
pub struct SingleFieldVariant {
    /// [`syn::Variant`] itself.
    variant: syn::Variant,

    /// Indicates, whether `#[event(init)]` attribute is present or not.
    is_initial: bool,
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
    /// - If [`VariantAttrs::init`] and [`VariantAttrs::ignore`] were specified
    ///   simultaneously.
    /// - If [`syn::Variant`] doesn't have exactly one unnamed 1 [`syn::Field`]
    ///   and is not ignored.
    fn parse_variant(
        variant: &syn::Variant,
    ) -> syn::Result<Option<SingleFieldVariant>> {
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

        Ok(Some(SingleFieldVariant {
            variant: variant.clone(),
            is_initial: attrs.init.is_some(),
        }))
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
    /// [0]: arcana_core::es::event::Event
    #[must_use]
    pub fn impl_event(&self) -> TokenStream {
        let ty = &self.ident;
        let (impl_gens, ty_gens, where_clause) = self.generics.split_for_impl();

        let var = self
            .variants
            .iter()
            .map(|v| &v.variant.ident)
            .collect::<Vec<_>>();

        let unreachable_arm = self.has_ignored_variants.then(|| {
            quote! { _ => unreachable!(), }
        });

        quote! {
            #[automatically_derived]
            impl #impl_gens ::arcana::es::Event for #ty #ty_gens #where_clause {
                fn name(&self) -> ::arcana::es::event::Name {
                    match self {
                        #(
                            Self::#var(f) => ::arcana::es::Event::name(f),
                        )*
                        #unreachable_arm
                    }
                }

                fn version(&self) -> ::arcana::es::event::Version {
                    match self {
                        #(
                            Self::#var(f) => ::arcana::es::Event::version(f),
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
    /// [0]: arcana_core::es::event::Sourced
    #[must_use]
    pub fn impl_event_sourced(&self) -> TokenStream {
        let ty = &self.ident;
        let (_, ty_gens, _) = self.generics.split_for_impl();
        let turbofish_gens = ty_gens.as_turbofish();

        let var_tys = self.variants.iter().map(|v| {
            let var_ty = v.variant.fields.iter().next().map(|f| &f.ty);
            if v.is_initial {
                quote! { ::arcana::es::event::Initial<#var_ty> }
            } else {
                quote! { #var_ty }
            }
        });

        let mut ext_gens = self.generics.clone();
        ext_gens.params.push(parse_quote! { __S });
        ext_gens.make_where_clause().predicates.push(parse_quote! {
            Self: #( ::arcana::es::event::Sourced<#var_tys> )+*
        });
        let (impl_gens, _, where_clause) = ext_gens.split_for_impl();

        let arms = self.variants.iter().map(|v| {
            let var = &v.variant.ident;
            let var_ty = v.variant.fields.iter().next().map(|f| &f.ty);

            let event = if v.is_initial {
                quote! {
                    <::arcana::es::event::Initial<#var_ty>
                        as ::arcana::es::event::codegen::ref_cast::RefCast
                    >::ref_cast(f)
                }
            } else {
                quote! { f }
            };
            quote! {
                #ty #turbofish_gens::#var(f) => {
                    ::arcana::es::event::Sourced::apply(self, #event);
                },
            }
        });
        let unreachable_arm = self.has_ignored_variants.then(|| {
            quote! { _ => unreachable!(), }
        });

        quote! {
            #[automatically_derived]
            impl #impl_gens ::arcana::es::event::Sourced<#ty #ty_gens>
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
            .flat_map(|v| &v.variant.fields)
            .map(|f| &f.ty)
            .collect::<Vec<_>>();

        // TODO: Use `Self::__arcana_events()` inside impl instead of type
        //       params substitution, once rust-lang/rust#57775 is resolved:
        //       https://github.com/rust-lang/rust/issues/57775
        let ty_subst_gens = Self::substitute_generics_trivially(&self.generics);

        let glue = quote! { ::arcana::es::event::codegen };
        quote! {
            #[automatically_derived]
            #[doc(hidden)]
            impl #impl_gens #glue::Versioned for #ty #ty_gens
                 #where_clause
            {
                #[doc(hidden)]
                const COUNT: usize =
                    #( <#var_ty as #glue::Versioned>::COUNT )+*;
            }

            #[automatically_derived]
            #[doc(hidden)]
            impl #ty #ty_gens {
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
            }

            #[automatically_derived]
            #[doc(hidden)]
            const _: () = ::std::assert!(
                !#glue::has_different_types_with_same_name_and_ver(
                    #ty::#ty_subst_gens::__arcana_events(),
                ),
                "having different `Event` types with the same name and version \
                 inside a single enum is forbidden",
            );
        }
    }

    /// Generates code to derive [`Transformer`][0] trait for any [wrapped][1]
    /// [`Adapter`][2], which can transform every enum variant.
    ///
    /// [0]: arcana_core::es::event::adapter::Transformer
    /// [1]: arcana_core::es::event::adapter::Adapted
    /// [2]: arcana_core::es::event::Adapter
    #[must_use]
    pub fn impl_transformer(&self) -> TokenStream {
        let event = &self.ident;

        let inner_match = self.inner_match();
        let transformed = self.transformed_stream();

        let generics = self.transformer_generics();
        let (impl_gen, _, where_clause) = generics.split_for_impl();
        let (_, type_gen, _) = self.generics.split_for_impl();

        let unreachable_arm = self
            .has_ignored_variants
            .then(|| quote! { _ => unreachable!(), });

        quote! {
            #[automatically_derived]
            impl #impl_gen ::arcana::es::event::adapter::Transformer<
                '__ctx, #event #type_gen, __Ctx
            > for ::arcana::es::event::adapter::Adapted<__A> #where_clause
            {
                type Error = <__A as ::arcana::es::event::adapter::Returning>::
                    Error;
                type Transformed =
                    <__A as ::arcana::es::event::adapter::Returning>::
                        Transformed;
                type TransformedStream<'out>
                where
                    '__ctx: 'out,
                    __A: 'out,
                    __Ctx: '__ctx + 'out,
                = #transformed;

                fn transform<'me, 'out>(
                    &'me self,
                    __event: #event #type_gen,
                    __context: &'__ctx __Ctx,
                ) -> <Self as ::arcana::es::event::adapter::
                    Transformer<'__ctx, #event #type_gen, __Ctx>>::
                        TransformedStream<'out>
                where
                    'me: 'out,
                    '__ctx: 'out,
                {
                    match __event {
                        #inner_match
                        #unreachable_arm
                    }
                }
            }
        }
    }

    /// Generates [`syn::Generics`] to for [wrapped][0] [`Adapter`][1], which
    /// [`transform`][2]s every enum variant.
    ///
    /// [0]: arcana_core::es::event::adapter::Adapted
    /// [1]: arcana_core::es::event::Adapter
    /// [2]: arcana_core::es::event::adapter::Transformer::transform
    #[must_use]
    pub fn transformer_generics(&self) -> syn::Generics {
        let mut generics = self.generics.clone();
        let var_type = self
            .variants
            .iter()
            .filter_map(|var| var.variant.fields.iter().next().map(|f| &f.ty))
            .collect::<Vec<_>>();

        let additional_generic_params: Punctuated<
            syn::GenericParam,
            syn::Token![,],
        > = parse_quote! {
            '__ctx, __A, __Ctx
        };
        let transformer_bounds: Punctuated<
            syn::WherePredicate,
            syn::Token![,],
        > = parse_quote! {
            __A: ::arcana::es::event::adapter::Returning,
            Self: #(
                ::arcana::es::event::adapter::Transformer<
                    '__ctx, #var_type, __Ctx
                >
            )+*,
            <__A as ::arcana::es::event::adapter::Returning>::Transformed:
                'static
                #(
                    + ::std::convert::From<<
                        Self as ::arcana::es::event::adapter::Transformer<
                            '__ctx, #var_type, __Ctx
                        >
                    >::Transformed>
                )*,
            <__A as ::arcana::es::event::adapter::Returning>::Error:
                'static
                #(
                    + ::std::convert::From<<
                        Self as ::arcana::es::event::adapter::Transformer<
                            '__ctx, #var_type, __Ctx
                        >
                    >::Error>
                )*,
            #(
                <Self as ::arcana::es::event::adapter::Transformer<
                    '__ctx, #var_type, __Ctx
                >>::Transformed: 'static,
                <Self as ::arcana::es::event::adapter::Transformer<
                    '__ctx, #var_type, __Ctx
                >>::Error: 'static,
            )*
        };

        generics.params.extend(additional_generic_params);
        generics
            .make_where_clause()
            .predicates
            .extend(transformer_bounds);

        generics
    }

    /// Generates code of [`Transformer::Transformed`][0] associated type.
    ///
    /// This is basically a recursive type
    /// [`Either`]`<Var1, `[`Either`]`<Var2, ...>>`, where every `VarN` is an
    /// enum variant's [`Transformer::TransformedStream`][1] wrapped in a
    /// [`stream::Map`] with a function that uses [`From`] impl to transform
    /// [`Event`]s into compatible ones.
    ///
    /// [0]: arcana_core::es::event::adapter::Transformer::Transformed
    /// [1]: arcana_core::es::event::adapter::Transformer::TransformedStream
    /// [`Either`]: futures::future::Either
    /// [`Event`]: trait@::arcana_core::es::Event
    /// [`stream::Map`]: futures::stream::Map
    #[must_use]
    pub fn transformed_stream(&self) -> TokenStream {
        let event = &self.ident;
        let (_, ty_gen, _) = self.generics.split_for_impl();

        let transformed_stream = |from: &syn::Type| {
            quote! {
                ::arcana::es::event::codegen::futures::stream::Map<
                    <Self as ::arcana::es::event::adapter::Transformer<
                        '__ctx, #from, __Ctx
                    >>::TransformedStream<'out>,
                    fn(
                        ::std::result::Result<
                            <Self as ::arcana::es::event::adapter::
                                Transformer<'__ctx, #from, __Ctx>>::
                                    Transformed,
                            <Self as ::arcana::es::event::adapter::
                                Transformer<'__ctx, #from, __Ctx>>::Error,
                        >,
                    ) -> ::std::result::Result<
                        <Self as ::arcana::es::event::adapter::
                            Transformer<'__ctx, #event #ty_gen, __Ctx>>::
                                Transformed,
                        <Self as ::arcana::es::event::adapter::
                            Transformer<'__ctx, #event #ty_gen, __Ctx>>::Error,
                    >
                >
            }
        };

        self.variants
            .iter()
            .filter_map(|var| var.variant.fields.iter().next().map(|f| &f.ty))
            .rev()
            .fold(None, |acc, ty| {
                let variant_stream = transformed_stream(ty);
                Some(
                    acc.map(|stream| {
                        quote! {
                            ::arcana::es::event::codegen::futures::future::
                            Either<
                                #variant_stream,
                                #stream,
                            >
                        }
                    })
                    .unwrap_or(variant_stream),
                )
            })
            .unwrap_or_default()
    }

    /// Generates code for implementation of a [`Transformer::transform()`][0]
    /// method.
    ///
    /// Generated code matches over every [`Event`]'s variant and makes it
    /// compatible with [`Definition::transformed_stream()`] type with
    /// [`StreamExt::left_stream()`] and [`StreamExt::right_stream()`]
    /// combinators.
    ///
    /// [0]: arcana_core::es::event::adapter::Transformer::transform
    /// [`Event`]: trait@arcana_core::es::Event
    /// [`StreamExt::left_stream()`]: futures::StreamExt::left_stream()
    /// [`StreamExt::right_stream()`]: futures::StreamExt::right_stream()
    #[must_use]
    pub fn inner_match(&self) -> TokenStream {
        let event = &self.ident;
        let (_, ty_gens, _) = self.generics.split_for_impl();
        let turbofish_gens = ty_gens.as_turbofish();

        self.variants
            .iter()
            .filter_map(|var| {
                var.variant
                    .fields
                    .iter()
                    .next()
                    .map(|f| (&var.variant.ident, &f.ty))
            })
            .enumerate()
            .map(|(i, (variant_ident, var_ty))| {
                let stream_map = quote! {
                    ::arcana::es::event::codegen::futures::StreamExt::map(
                        <Self as ::arcana::es::event::adapter::Transformer<
                            '__ctx, #var_ty, __Ctx
                        > >::transform(self, __event, __context),
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
                    ::arcana::es::event::codegen::futures::StreamExt::
                    right_stream
                };
                let left_stream = quote! {
                    ::arcana::es::event::codegen::futures::StreamExt::
                    left_stream
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
                    #event #turbofish_gens::#variant_ident(__event) => {
                        #transformed_stream
                    },
                }
            })
            .collect()
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
            impl ::arcana::es::Event for Event {
                fn name(&self) -> ::arcana::es::event::Name {
                    match self {
                        Self::File(f) => ::arcana::es::Event::name(f),
                        Self::Chat(f) => ::arcana::es::Event::name(f),
                    }
                }

                fn version(&self) -> ::arcana::es::event::Version {
                    match self {
                        Self::File(f) => ::arcana::es::Event::version(f),
                        Self::Chat(f) => ::arcana::es::Event::version(f),
                    }
                }
            }

            #[automatically_derived]
            impl<__S> ::arcana::es::event::Sourced<Event> for Option<__S>
            where
                Self: ::arcana::es::event::Sourced<
                          ::arcana::es::event::Initial<FileEvent>
                      > +
                      ::arcana::es::event::Sourced<ChatEvent>
            {
                fn apply(&mut self, event: &Event) {
                    match event {
                        Event::File(f) => {
                            ::arcana::es::event::Sourced::apply(
                                self,
                                <::arcana::es::event::Initial<FileEvent> as
                                ::arcana::es::event::codegen::ref_cast::RefCast
                                >::ref_cast(f)
                            );
                        },
                        Event::Chat(f) => {
                            ::arcana::es::event::Sourced::apply(self, f);
                        },
                    }
                }
            }

            #[automatically_derived]
            impl<'__ctx, __A, __Ctx> ::arcana::es::event::adapter::Transformer<
                '__ctx, Event, __Ctx
            > for ::arcana::es::event::adapter::Adapted<__A>
            where
                __A: ::arcana::es::event::adapter::Returning,
                Self:
                    ::arcana::es::event::adapter::Transformer<
                        '__ctx, FileEvent, __Ctx
                    > +
                    ::arcana::es::event::adapter::Transformer<
                        '__ctx, ChatEvent, __Ctx
                    >,
                <__A as ::arcana::es::event::adapter::Returning>::Transformed:
                    'static +
                    ::std::convert::From< <
                        Self as ::arcana::es::event::adapter::Transformer<
                            '__ctx, FileEvent, __Ctx
                        >
                    >::Transformed> +
                    ::std::convert::From< <
                        Self as ::arcana::es::event::adapter::Transformer<
                            '__ctx, ChatEvent, __Ctx
                        >
                    >::Transformed>,
                <__A as ::arcana::es::event::adapter::Returning>::Error:
                    'static +
                    ::std::convert::From< <
                        Self as ::arcana::es::event::adapter::Transformer<
                            '__ctx, FileEvent, __Ctx
                        >
                    >::Error> +
                    ::std::convert::From< <
                        Self as ::arcana::es::event::adapter::Transformer<
                            '__ctx, ChatEvent, __Ctx
                        >
                    >::Error>,
                <Self as ::arcana::es::event::adapter::
                    Transformer<'__ctx, FileEvent, __Ctx> >::Transformed:
                        'static,
                <Self as ::arcana::es::event::adapter::
                    Transformer<'__ctx, FileEvent, __Ctx> >::Error:
                        'static,
                <Self as ::arcana::es::event::adapter::
                    Transformer<'__ctx, ChatEvent, __Ctx> >::Transformed:
                        'static,
                <Self as ::arcana::es::event::adapter::
                    Transformer<'__ctx, ChatEvent, __Ctx> >::Error:
                        'static
            {
                type Error = <__A as ::arcana::es::event::adapter::Returning>::
                    Error;
                type Transformed =
                    <__A as ::arcana::es::event::adapter::Returning>::
                        Transformed;
                type TransformedStream<'out>
                where
                    '__ctx: 'out,
                    __A: 'out,
                    __Ctx: '__ctx + 'out,
                =
                    ::arcana::es::event::codegen::futures::future::Either<
                        ::arcana::es::event::codegen::futures::stream::Map<
                            <Self as ::arcana::es::event::adapter::Transformer<
                                '__ctx, FileEvent, __Ctx
                            >>::TransformedStream<'out>,
                            fn(
                                ::std::result::Result<
                                    <Self as ::arcana::es::event::adapter::
                                             Transformer<
                                                '__ctx, FileEvent, __Ctx
                                             >>::Transformed,
                                    <Self as ::arcana::es::event::adapter::
                                             Transformer<
                                                '__ctx, FileEvent, __Ctx
                                             >>::Error,
                                >,
                            ) -> ::std::result::Result<
                                <Self as ::arcana::es::event::adapter::
                                         Transformer<'__ctx, Event, __Ctx>>::
                                         Transformed,
                                <Self as ::arcana::es::event::adapter::
                                         Transformer<'__ctx, Event, __Ctx>>::
                                         Error,
                            >
                        >,
                        ::arcana::es::event::codegen::futures::stream::Map<
                            <Self as ::arcana::es::event::adapter::Transformer<
                                '__ctx, ChatEvent, __Ctx
                            >>::TransformedStream<'out>,
                            fn(
                                ::std::result::Result<
                                    <Self as ::arcana::es::event::adapter::
                                             Transformer<
                                                '__ctx, ChatEvent, __Ctx
                                             >>::Transformed,
                                    <Self as ::arcana::es::event::adapter::
                                             Transformer<
                                                '__ctx, ChatEvent, __Ctx
                                             >>::Error,
                                >,
                            ) -> ::std::result::Result<
                                <Self as ::arcana::es::event::adapter::
                                         Transformer<'__ctx, Event, __Ctx>>::
                                         Transformed,
                                <Self as ::arcana::es::event::adapter::
                                         Transformer<'__ctx, Event, __Ctx>>::
                                         Error,
                            >
                        >,
                    >;

                fn transform<'me, 'out>(
                    &'me self,
                    __event: Event,
                    __context: &'__ctx __Ctx,
                ) -> <Self as ::arcana::es::event::adapter::
                              Transformer<'__ctx, Event, __Ctx>>::
                              TransformedStream<'out>
                where
                    'me: 'out,
                    '__ctx: 'out,
                {
                    match __event {
                        Event::File(__event) => {
                            ::arcana::es::event::codegen::futures::StreamExt::
                                left_stream(
                                ::arcana::es::event::codegen::futures::
                                StreamExt::map(
                                    <Self as ::arcana::es::event::adapter::
                                             Transformer<
                                                '__ctx, FileEvent, __Ctx
                                             >
                                    >::transform(self, __event, __context),
                                    {
                                        let __transform_fn: fn(_) -> _ =
                                        |__res| {
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
                            )
                        },
                        Event::Chat(__event) => {
                            ::arcana::es::event::codegen::futures::StreamExt::
                            right_stream(
                                ::arcana::es::event::codegen::futures::
                                StreamExt::map(
                                    <Self as ::arcana::es::event::adapter::
                                             Transformer<
                                                '__ctx, ChatEvent, __Ctx
                                             >
                                    >::transform(self, __event, __context),
                                    {
                                        let __transform_fn: fn(_) -> _ =
                                        |__res| {
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
                            )
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
                        let events = <FileEvent>::__arcana_events();
                        let mut j = 0;
                        while j < events.len() {
                            res[i] = events[j];
                            j += 1;
                            i += 1;
                        }
                    }
                    {
                        let events = <ChatEvent>::__arcana_events();
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

            #[automatically_derived]
            #[doc(hidden)]
            const _: () = ::std::assert!(
                !::arcana::es::event::codegen::
                    has_different_types_with_same_name_and_ver(
                        Event::<>::__arcana_events(),
                    ),
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
            impl<'a, F, C> ::arcana::es::Event for Event<'a, F, C> {
                fn name(&self) -> ::arcana::es::event::Name {
                    match self {
                        Self::File(f) => ::arcana::es::Event::name(f),
                        Self::Chat(f) => ::arcana::es::Event::name(f),
                    }
                }

                fn version(&self) -> ::arcana::es::event::Version {
                    match self {
                        Self::File(f) => ::arcana::es::Event::version(f),
                        Self::Chat(f) => ::arcana::es::Event::version(f),
                    }
                }
            }

            #[automatically_derived]
            impl<'a, F, C, __S> ::arcana::es::event::Sourced<Event<'a, F, C> >
                for Option<__S>
            where
                Self: ::arcana::es::event::Sourced<
                          ::arcana::es::event::Initial<FileEvent<'a, F> >
                      > +
                      ::arcana::es::event::Sourced<ChatEvent<'a, C> >
            {
                fn apply(&mut self, event: &Event<'a, F, C>) {
                    match event {
                        Event::<'a, F, C>::File(f) => {
                            ::arcana::es::event::Sourced::apply(
                                self,
                                <::arcana::es::event::Initial<FileEvent<'a, F> >
                                    as ::arcana::es::event::codegen::ref_cast::
                                        RefCast
                                >::ref_cast(f)
                            );
                        },
                        Event::<'a, F, C>::Chat(f) => {
                            ::arcana::es::event::Sourced::apply(self, f);
                        },
                    }
                }
            }

            #[automatically_derived]
            impl<'a, '__ctx, F, C, __A, __Ctx> ::arcana::es::event::adapter::
                Transformer<'__ctx, Event<'a, F, C>, __Ctx> for
                    ::arcana::es::event::adapter::Adapted<__A>
            where
                __A: ::arcana::es::event::adapter::Returning,
                Self:
                   ::arcana::es::event::adapter::Transformer<
                        '__ctx, FileEvent<'a, F>, __Ctx
                   > +
                   ::arcana::es::event::adapter::Transformer<
                        '__ctx, ChatEvent<'a, C>, __Ctx
                   >,
                <__A as ::arcana::es::event::adapter::Returning>::Transformed:
                    'static +
                    ::std::convert::From< <
                        Self as ::arcana::es::event::adapter::Transformer<
                            '__ctx, FileEvent<'a, F>, __Ctx>
                        >::Transformed> +
                    ::std::convert::From< <
                        Self as ::arcana::es::event::adapter::Transformer<
                            '__ctx, ChatEvent<'a, C>, __Ctx>
                        >::Transformed>,
                <__A as ::arcana::es::event::adapter::Returning>::Error:
                    'static +
                    ::std::convert::From< <
                        Self as ::arcana::es::event::adapter::Transformer<
                            '__ctx, FileEvent<'a, F>, __Ctx
                        >
                    >::Error> +
                    ::std::convert::From< <
                        Self as ::arcana::es::event::adapter::Transformer<
                            '__ctx, ChatEvent<'a, C>, __Ctx
                        >
                    >::Error>,
                <Self as ::arcana::es::event::adapter::
                    Transformer<'__ctx, FileEvent<'a, F>, __Ctx> >::Transformed:
                        'static,
                <Self as ::arcana::es::event::adapter::
                    Transformer<'__ctx, FileEvent<'a, F>, __Ctx> >::Error:
                        'static,
                <Self as ::arcana::es::event::adapter::
                    Transformer<'__ctx, ChatEvent<'a, C>, __Ctx> >::Transformed:
                        'static,
                <Self as ::arcana::es::event::adapter::
                    Transformer<'__ctx, ChatEvent<'a, C>, __Ctx> >::Error:
                        'static
            {
                type Error = <__A as ::arcana::es::event::adapter::Returning>::
                    Error;
                type Transformed =
                    <__A as ::arcana::es::event::adapter::Returning>::
                        Transformed;
                type TransformedStream<'out>
                where
                    '__ctx: 'out,
                    __A: 'out,
                    __Ctx: '__ctx + 'out,
                =
                    ::arcana::es::event::codegen::futures::future::Either<
                        ::arcana::es::event::codegen::futures::stream::Map<
                            <Self as ::arcana::es::event::adapter::Transformer<
                                '__ctx, FileEvent<'a, F>, __Ctx
                            >>::TransformedStream<'out>,
                            fn(
                                ::std::result::Result<
                                    <Self as ::arcana::es::event::adapter::
                                             Transformer<
                                                '__ctx, FileEvent<'a, F>, __Ctx
                                             >>::Transformed,
                                    <Self as ::arcana::es::event::adapter::
                                             Transformer<
                                                '__ctx, FileEvent<'a, F>, __Ctx
                                             >>::Error,
                                >,
                            ) -> ::std::result::Result<
                                <Self as ::arcana::es::event::adapter::
                                         Transformer<
                                            '__ctx, Event<'a, F, C>, __Ctx
                                         >>::Transformed,
                                <Self as ::arcana::es::event::adapter::
                                         Transformer<
                                             '__ctx, Event<'a, F, C>, __Ctx
                                         >>::Error,
                            >
                        >,
                        ::arcana::es::event::codegen::futures::stream::Map<
                            <Self as ::arcana::es::event::adapter::Transformer<
                                '__ctx, ChatEvent<'a, C>, __Ctx
                            >>::TransformedStream<'out>,
                            fn(
                                ::std::result::Result<
                                    <Self as ::arcana::es::event::adapter::
                                             Transformer<
                                                '__ctx, ChatEvent<'a, C>, __Ctx
                                             >>::Transformed,
                                    <Self as ::arcana::es::event::adapter::
                                             Transformer<
                                                '__ctx, ChatEvent<'a, C>, __Ctx
                                             >>::Error,
                                >,
                            ) -> ::std::result::Result<
                                <Self as ::arcana::es::event::adapter::
                                         Transformer<
                                            '__ctx, Event<'a, F, C>, __Ctx
                                         >>::Transformed,
                                <Self as ::arcana::es::event::adapter::
                                         Transformer<
                                            '__ctx, Event<'a, F, C>, __Ctx
                                         >>::Error,
                            >
                        >,
                    >;

                fn transform<'me, 'out>(
                    &'me self,
                    __event: Event<'a, F, C>,
                    __context: &'__ctx __Ctx,
                ) -> <Self as ::arcana::es::event::adapter::
                              Transformer<'__ctx, Event<'a, F, C>, __Ctx>>::
                              TransformedStream<'out>
                where
                    'me: 'out,
                    '__ctx: 'out,
                {
                    match __event {
                        Event::<'a, F, C>::File(__event) => {
                            ::arcana::es::event::codegen::futures::StreamExt::
                                left_stream(
                                ::arcana::es::event::codegen::futures::
                                StreamExt::map(
                                    <Self as ::arcana::es::event::adapter::
                                    Transformer<
                                        '__ctx, FileEvent<'a, F>, __Ctx
                                    > >::transform(self, __event, __context),
                                    {
                                        let __transform_fn: fn(_) -> _ =
                                        |__res| {
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
                            )
                        },
                        Event::<'a, F, C>::Chat(__event) => {
                            ::arcana::es::event::codegen::futures::StreamExt::
                            right_stream(
                                ::arcana::es::event::codegen::futures::
                                StreamExt::map(
                                    <Self as ::arcana::es::event::adapter::
                                    Transformer<
                                        '__ctx, ChatEvent<'a, C>, __Ctx
                                    > >::transform(self, __event, __context),
                                    {
                                        let __transform_fn: fn(_) -> _ =
                                        |__res| {
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
                            )
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
                        let events = <FileEvent<'a, F> >::__arcana_events();
                        let mut j = 0;
                        while j < events.len() {
                            res[i] = events[j];
                            j += 1;
                            i += 1;
                        }
                    }
                    {
                        let events = <ChatEvent<'a, C> >::__arcana_events();
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

            #[automatically_derived]
            #[doc(hidden)]
            const _: () = ::std::assert!(
                !::arcana::es::event::codegen::
                    has_different_types_with_same_name_and_ver(
                        Event::<'static, (), ()>::__arcana_events(),
                    ),
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
            impl ::arcana::es::Event for Event {
                fn name(&self) -> ::arcana::es::event::Name {
                    match self {
                        Self::File(f) => ::arcana::es::Event::name(f),
                        Self::Chat(f) => ::arcana::es::Event::name(f),
                        _ => unreachable!(),
                    }
                }

                fn version(&self) -> ::arcana::es::event::Version {
                    match self {
                        Self::File(f) => ::arcana::es::Event::version(f),
                        Self::Chat(f) => ::arcana::es::Event::version(f),
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
                            ::arcana::es::event::Sourced::apply(self, f);
                        },
                        Event::Chat(f) => {
                            ::arcana::es::event::Sourced::apply(self, f);
                        },
                        _ => unreachable!(),
                    }
                }
            }

            #[automatically_derived]
            impl<'__ctx, __A, __Ctx> ::arcana::es::event::adapter::Transformer<
                '__ctx, Event, __Ctx
            > for ::arcana::es::event::adapter::Adapted<__A>
            where
                __A: ::arcana::es::event::adapter::Returning,
                Self:
                    ::arcana::es::event::adapter::Transformer<
                        '__ctx, FileEvent, __Ctx
                    > +
                    ::arcana::es::event::adapter::Transformer<
                        '__ctx, ChatEvent, __Ctx
                    >,
                <__A as ::arcana::es::event::adapter::Returning>::Transformed:
                    'static +
                    ::std::convert::From< <
                        Self as ::arcana::es::event::adapter::Transformer<
                            '__ctx, FileEvent, __Ctx
                        >
                    >::Transformed> +
                    ::std::convert::From< <
                        Self as ::arcana::es::event::adapter::Transformer<
                            '__ctx, ChatEvent, __Ctx
                        >
                    >::Transformed>,
                <__A as ::arcana::es::event::adapter::Returning>::Error:
                    'static +
                    ::std::convert::From< <
                        Self as ::arcana::es::event::adapter::Transformer<
                            '__ctx, FileEvent, __Ctx
                        >
                    >::Error> +
                    ::std::convert::From< <
                        Self as ::arcana::es::event::adapter::Transformer<
                            '__ctx, ChatEvent, __Ctx
                        >
                    >::Error>,
                <Self as ::arcana::es::event::adapter::
                    Transformer<'__ctx, FileEvent, __Ctx> >::Transformed:
                        'static,
                <Self as ::arcana::es::event::adapter::
                    Transformer<'__ctx, FileEvent, __Ctx> >::Error: 'static,
                <Self as ::arcana::es::event::adapter::
                    Transformer<'__ctx, ChatEvent, __Ctx> >::Transformed:
                        'static,
                <Self as ::arcana::es::event::adapter::
                    Transformer<'__ctx, ChatEvent, __Ctx> >::Error: 'static
            {
                type Error = <__A as ::arcana::es::event::adapter::Returning>::
                    Error;
                type Transformed =
                    <__A as ::arcana::es::event::adapter::Returning>::
                        Transformed;
                type TransformedStream<'out>
                where
                    '__ctx: 'out,
                    __A: 'out,
                    __Ctx: '__ctx + 'out,
                =
                    ::arcana::es::event::codegen::futures::future::Either<
                        ::arcana::es::event::codegen::futures::stream::Map<
                            <Self as ::arcana::es::event::adapter::Transformer<
                                '__ctx, FileEvent, __Ctx
                            >>::TransformedStream<'out>,
                            fn(
                                ::std::result::Result<
                                    <Self as ::arcana::es::event::adapter::
                                             Transformer<
                                                '__ctx, FileEvent, __Ctx
                                             >>::Transformed,
                                    <Self as ::arcana::es::event::adapter::
                                             Transformer<
                                                '__ctx, FileEvent, __Ctx
                                             >>::Error,
                                >,
                            ) -> ::std::result::Result<
                                <Self as ::arcana::es::event::adapter::
                                         Transformer<'__ctx, Event, __Ctx>>::
                                         Transformed,
                                <Self as ::arcana::es::event::adapter::
                                         Transformer<'__ctx, Event, __Ctx>>::
                                         Error,
                            >
                        >,
                        ::arcana::es::event::codegen::futures::stream::Map<
                            <Self as ::arcana::es::event::adapter::Transformer<
                                '__ctx, ChatEvent, __Ctx
                            >>::TransformedStream<'out>,
                            fn(
                                ::std::result::Result<
                                    <Self as ::arcana::es::event::adapter::
                                             Transformer<
                                                '__ctx, ChatEvent, __Ctx
                                             >>::Transformed,
                                    <Self as ::arcana::es::event::adapter::
                                             Transformer<
                                                '__ctx, ChatEvent, __Ctx
                                             >>::Error,
                                >,
                            ) -> ::std::result::Result<
                                <Self as ::arcana::es::event::adapter::
                                         Transformer<'__ctx, Event, __Ctx>>::
                                         Transformed,
                                <Self as ::arcana::es::event::adapter::
                                         Transformer<'__ctx, Event, __Ctx>>::
                                         Error,
                            >
                        >,
                    >;

                fn transform<'me, 'out>(
                    &'me self,
                    __event: Event,
                    __context: &'__ctx __Ctx,
                ) -> <Self as ::arcana::es::event::adapter::
                              Transformer<'__ctx, Event, __Ctx>>::
                              TransformedStream<'out>
                where
                    'me: 'out,
                    '__ctx: 'out,
                {
                    match __event {
                        Event::File(__event) => {
                            ::arcana::es::event::codegen::futures::StreamExt::
                                left_stream(
                                ::arcana::es::event::codegen::futures::
                                StreamExt::map(
                                    <Self as ::arcana::es::event::adapter::
                                             Transformer<
                                                 '__ctx, FileEvent, __Ctx
                                             >
                                    >::transform(self, __event, __context),
                                    {
                                        let __transform_fn: fn(_) -> _ =
                                        |__res| {
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
                            )
                        },
                        Event::Chat(__event) => {
                            ::arcana::es::event::codegen::futures::StreamExt::
                            right_stream(
                                ::arcana::es::event::codegen::futures::
                                StreamExt::map(
                                    <Self as ::arcana::es::event::adapter::
                                             Transformer<
                                                 '__ctx, ChatEvent, __Ctx
                                             >
                                    >::transform(self, __event, __context),
                                    {
                                        let __transform_fn: fn(_) -> _ =
                                        |__res| {
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
                            )
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
                        let events = <FileEvent>::__arcana_events();
                        let mut j = 0;
                        while j < events.len() {
                            res[i] = events[j];
                            j += 1;
                            i += 1;
                        }
                    }
                    {
                        let events = <ChatEvent>::__arcana_events();
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

            #[automatically_derived]
            #[doc(hidden)]
            const _: () = ::std::assert!(
                !::arcana::es::event::codegen::
                    has_different_types_with_same_name_and_ver(
                        Event::<>::__arcana_events(),
                    ),
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
