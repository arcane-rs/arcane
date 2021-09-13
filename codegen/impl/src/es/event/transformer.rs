//! `#[derive(adapter::Transformer)]` macro implementation.

use std::{convert::TryFrom, iter};

use either::Either;
use proc_macro2::{Span, TokenStream};
use quote::quote;
use std::{num::NonZeroUsize, ops::Deref};
use syn::{parse_quote, spanned::Spanned};
use synthez::{ParseAttrs, Required, Spanning, ToTokens};

/// Expands `#[derive(adapter::Transformer)]` macro.
///
/// # Errors
///
/// If failed to parse [`Attrs`] or transform them into [`Definition`].
pub fn derive(input: TokenStream) -> syn::Result<TokenStream> {
    let input = syn::parse2::<syn::DeriveInput>(input)?;
    let definition = Definition::try_from(input)?;

    Ok(quote! { #definition })
}

/// Helper attributes of `#[derive(adapter::Transformer)]` macro.
#[derive(Debug, Default, ParseAttrs)]
pub struct Attrs {
    /// [`Vec`] of [`InnerAttrs`] for generating [`Transformer`][0] trait impls.
    ///
    /// [0]: arcana_core::es::adapter::Transformer
    #[parse(nested)]
    pub transformer: Vec<Spanning<InnerAttrs>>,
}

/// Inner attributes of `#[event(transformer(...)]`.
///
/// Each of them used to generate [`Transformer`][0] trait impl.
///
/// [0]: arcana_core::es::adapter::Transformer
#[derive(Debug, Default, ParseAttrs)]
pub struct InnerAttrs {
    /// [`Transformer`][0] generic types.
    ///
    /// [0]: arcana_core::es::adapter::Transformer
    #[parse(value, alias(from, event))]
    pub events: Vec<syn::Type>,

    /// [`Transformer::Transformed`][0] type.
    ///
    /// [0]: arcana_core::es::adapter::Transformer::Transformed
    #[parse(value, alias = into)]
    pub transformed: Required<syn::Type>,

    /// [`Transformer::Context`][0] type.
    ///
    /// [0]: arcana_core::es::adapter::Transformer::Context
    #[parse(value, alias = ctx)]
    pub context: Required<syn::Type>,

    /// [`Transformer::Error`][0] type.
    ///
    /// [0]: arcana_core::es::adapter::Transformer::Error
    #[parse(value, alias = err)]
    pub error: Required<syn::Type>,

    /// Maximum number of allowed [`Self::events`] enum variants.
    //  TODO: remove this restrictions, once we will be able to constantly
    //        traverse const array
    #[parse(value, validate = can_parse_as_non_zero_usize)]
    pub max_number_of_variants: Option<syn::LitInt>,
}

/// Checks whether the given `value` can be parsed as [`NonZeroUsize`].
fn can_parse_as_non_zero_usize<'a>(
    val: impl Into<Option<&'a syn::LitInt>>,
) -> syn::Result<()> {
    val.into()
        .map(syn::LitInt::base10_parse::<NonZeroUsize>)
        .transpose()
        .map(drop)
}

/// Default value for [`ImplDefinition::max_number_of_variants`].
pub const MAX_NUMBER_OF_VARIANTS: usize = 50;

impl InnerAttrs {
    /// Transforms [`InnerAttrs`] into [`ImplDefinition`]s.
    ///
    /// # Errors
    ///
    /// - If [`InnerAttrs::events`] is empty;
    /// - If failed to parse [`Self::max_number_of_variants`] into
    ///   [`NonZeroUsize`].
    pub fn into_impl_definition(
        self,
    ) -> impl Iterator<Item = syn::Result<ImplDefinition>> {
        let InnerAttrs {
            events: event,
            transformed,
            context,
            error,
            max_number_of_variants,
        } = self;

        if event.is_empty() {
            return Either::Left(iter::once(Err(syn::Error::new(
                transformed.span(),
                "expected at least 1 `event` or `from` attribute",
            ))));
        }

        Either::Right(event.into_iter().map(move |ev| {
            Ok(ImplDefinition {
                event: ev,
                transformed: transformed.deref().clone(),
                context: context.deref().clone(),
                error: error.deref().clone(),
                max_number_of_variants: max_number_of_variants
                    .as_ref()
                    .map_or(Ok(MAX_NUMBER_OF_VARIANTS), |max| {
                        max.base10_parse::<NonZeroUsize>()
                            .map(NonZeroUsize::get)
                    })?,
            })
        }))
    }
}

// TODO: add PartialEq impls in synthez
impl PartialEq for InnerAttrs {
    fn eq(&self, other: &Self) -> bool {
        *self.events == *other.events
            && *self.transformed == *other.transformed
            && *self.context == *other.context
            && *self.error == *other.error
            && self.max_number_of_variants == other.max_number_of_variants
    }
}

/// Representation of a type for implementing [`Transformer`][0], used for code
/// generation.
///
/// [0]: arcana_core::es::adapter::Transformer
#[derive(Debug, ToTokens)]
#[to_tokens(append(derive_transformers))]
pub struct Definition {
    /// [`syn::Ident`](struct@syn::Ident) of type [`Transformer`][0] is derived
    /// on.
    ///
    /// [0]: arcana_core::es::adapter::Transformer
    pub adapter: syn::Ident,

    /// [`syn::Generics`] of this enum's type.
    pub generics: syn::Generics,

    /// Definitions of structures to derive [`Transformer`][0] on.
    ///
    /// [0]: arcana_core::es::adapter::Transformer
    pub transformers: Vec<ImplDefinition>,
}

/// Representation of [`Transformer`][0] impl, used for code generation.
///
/// [0]: arcana_core::es::adapter::Transformer
#[derive(Debug)]
pub struct ImplDefinition {
    /// [`Transformer`][0] generic type.
    ///
    /// [0]: arcana_core::es::adapter::Transformer
    pub event: syn::Type,

    /// [`Transformer::Transformed`][0] type.
    ///
    /// [0]: arcana_core::es::adapter::Transformer::Transformed
    pub transformed: syn::Type,

    /// [`Transformer::Context`][0] type.
    ///
    /// [0]: arcana_core::es::adapter::Transformer::Context
    pub context: syn::Type,

    /// [`Transformer::Error`][0] type.
    ///
    /// [0]: arcana_core::es::adapter::Transformer::Error
    pub error: syn::Type,

    /// Maximum number of allowed [`Self::event`] enum variants.
    pub max_number_of_variants: usize,
}

impl TryFrom<syn::DeriveInput> for Definition {
    type Error = syn::Error;

    fn try_from(input: syn::DeriveInput) -> syn::Result<Self> {
        let attrs: Attrs = Attrs::parse_attrs("event", &input)?;

        if attrs.transformer.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                "expected at least 1 `#[event(transformer(...))` attribute",
            ));
        }

        let transformers = attrs
            .transformer
            .into_iter()
            .flat_map(|tr| tr.into_inner().into_impl_definition())
            .collect::<Result<_, _>>()?;

        Ok(Self {
            adapter: input.ident,
            generics: input.generics,
            transformers,
        })
    }
}

impl Definition {
    /// Generates code to derive [`Transformer`][0] traits.
    ///
    /// [0]: arcana_core::es::adapter::Transformer
    #[must_use]
    pub fn derive_transformers(&self) -> TokenStream {
        let adapter = &self.adapter;
        let (impl_gen, type_gen, where_clause) = self.generics.split_for_impl();
        let codegen_path = quote! { ::arcana::es::event::codegen };
        let specialization_path = quote! {
            ::arcana::es::adapter::transformer::specialization
        };

        self.transformers.iter().map(|tr| {
            let transform_event = tr.transform_event(adapter);

            let ImplDefinition {
                event,
                transformed,
                context,
                error,
                max_number_of_variants,
            } = tr;

            let max = *max_number_of_variants;

            quote! {
                ::arcana::es::adapter::transformer::too_many_variants_in_enum!(
                    <#event as #codegen_path::EnumSize>::SIZE <= #max
                );

                #[automatically_derived]
                impl#impl_gen ::arcana::es::adapter::Transformer<#event> for
                    #adapter#type_gen #where_clause
                {
                    type Context = #context;
                    type Error = #error;
                    type Transformed = #transformed;
                    #[allow(clippy::type_complexity)]
                    type TransformedStream<'out> =
                        ::std::pin::Pin<
                            ::std::boxed::Box<
                                dyn #codegen_path::futures::Stream<
                                    Item = ::std::result::Result<
                                        Self::Transformed,
                                        Self::Error,
                                    >
                                > + 'out
                            >
                        >;

                    #[allow(clippy::modulo_one)]
                    fn transform<'me, 'ctx, 'out>(
                        &'me self,
                        event: #event,
                        ctx: &'ctx Self::Context,
                    ) -> Self::TransformedStream<'out>
                    where
                        'me: 'out,
                        'ctx: 'out,
                    {
                        #[allow(unused_imports)]
                        use #specialization_path::{
                            TransformedBySkipAdapter as _,
                            TransformedByAdapter as _,
                            TransformedByFrom as _,
                            TransformedByFromInitial as _,
                            TransformedByFromUpcast as _,
                            TransformedByFromInitialUpcast as _,
                            TransformedByEmpty as _,
                            Wrap,
                        };

                        #transform_event
                    }
                }
            }
        })
            .collect()
    }
}

impl ImplDefinition {
    /// Generates code which matches [`Event`] value with it's variant, checks
    /// whether [`Versioned`] or [`TransformedBy`] is implemented for it and
    /// then applies [`specialization`][0] transformations to it.
    ///
    /// Match is done by iterating `0..max_number_of_variants` over [`Get`]
    /// trait. As before that we asserted that number of variants is less or
    /// equals then `max_number_of_variants`, [`unreachable`] is really
    /// unreachable.
    ///
    /// [0]: arcana_core::es::adapter::transformer::specialization
    /// [`Event`]: arcana_core::es::Event
    /// [`Get`]: arcana_core::es::event::codegen::Get
    /// [`TransformedBy`]: arcana_core::es::adapter::TransformedBy
    /// [`Versioned`]: arcana_core::es::event::Versioned
    #[must_use]
    pub fn transform_event(&self, adapter: &syn::Ident) -> TokenStream {
        let codegen_path = quote! { ::arcana::es::event::codegen };

        let assert_versioned_or_transformed = Self::assert_impl_any(
            &syn::Ident::new("event", Span::call_site()),
            [
                parse_quote! { ::arcana::es::event::Versioned },
                parse_quote! { ::arcana::es::adapter::TransformedBy<#adapter> },
            ],
        );

        let ImplDefinition {
            event,
            transformed,
            max_number_of_variants,
            ..
        } = self;

        let max = *max_number_of_variants;
        let id = 0..max;
        quote! {
            #( if ::std::option::Option::is_some(
                &#codegen_path::Get::<{
                    #id % <#event as #codegen_path::EnumSize>::SIZE
                }>::get(&event)
            ) {
                let event = #codegen_path::Get::<{
                    #id % <#event as #codegen_path::EnumSize>::SIZE
                }>::unwrap(event);
                let check = #assert_versioned_or_transformed;
                let event = check();

                ::std::boxed::Box::pin(
                    (&&&&&&&Wrap::<&#adapter, _, #transformed>(
                        self,
                        &event,
                        ::std::marker::PhantomData,
                    ))
                        .get_tag()
                        .transform_event(self, event, ctx),
                )
            } else )*
            {
                unreachable!()
            }
        }
    }

    /// Generates closure, which moves `value` inside, asserts whether any of
    /// provided `traits` are implemented for it (if not, fails at compile time)
    /// and then returns `value`.
    ///
    /// We can't use closure which takes `value` as parameter as type inference
    /// can break assertion.
    #[must_use]
    pub fn assert_impl_any(
        value: &syn::Ident,
        traits: impl AsRef<[syn::Type]>,
    ) -> TokenStream {
        let traits = traits.as_ref().iter();

        quote! {
            || {
                struct AssertImplAnyFallback;

                struct ActualAssertImplAnyToken;
                trait AssertImplAnyToken {}
                impl AssertImplAnyToken for ActualAssertImplAnyToken {}

                fn assert_impl_any_token<T>(_: T)
                where T: AssertImplAnyToken {}

                let previous = AssertImplAnyFallback;

                #( let previous = {
                    struct Wrapper<T, N>(
                        ::std::marker::PhantomData<T>,
                        N,
                    );

                    impl<T, N> ::std::ops::Deref for Wrapper<T, N> {
                        type Target = N;

                        fn deref(&self) -> &Self::Target {
                            &self.1
                        }
                    }

                    impl<T, N> Wrapper<T, N> {
                        fn new(_: &T, right: N) -> Wrapper<T, N> {
                            Self(::std::marker::PhantomData, right)
                        }
                    }

                    impl<T, N> Wrapper<T, N>
                    where
                        T: #traits,
                    {
                        fn _static_assertions_impl_any(
                            &self,
                        ) -> ActualAssertImplAnyToken {
                            ActualAssertImplAnyToken
                        }
                    }

                    Wrapper::new(&#value, previous)
                }; )*

                assert_impl_any_token(
                    previous._static_assertions_impl_any(),
                );

                #value
            }
        }
    }
}

#[cfg(test)]
mod spec {
    use quote::quote;
    use syn::parse_quote;

    #[allow(clippy::too_many_lines)]
    #[test]
    fn derives_impl() {
        let input = parse_quote! {
            #[event(
                transformer(
                    from = Event,
                    into = IntoEvent,
                    context = dyn Any,
                    error = Infallible,
                    max_number_of_variants = 2,
                ),
            )]
            struct Adapter;
        };

        let output = quote! {
            ::arcana::es::adapter::transformer::too_many_variants_in_enum!(
                <Event as ::arcana::es::event::codegen::EnumSize>::SIZE <=
                    2usize
            );

            #[automatically_derived]
            impl ::arcana::es::adapter::Transformer<Event> for Adapter {
                type Context = dyn Any;
                type Error = Infallible;
                type Transformed = IntoEvent;
                #[allow(clippy::type_complexity)]
                type TransformedStream<'out> =
                    ::std::pin::Pin<
                        ::std::boxed::Box<
                            dyn ::arcana::es::event::codegen::futures::Stream<
                                Item = ::std::result::Result<
                                    Self::Transformed,
                                    Self::Error,
                                >
                            > + 'out
                        >
                    >;

                #[allow(clippy::modulo_one)]
                fn transform<'me, 'ctx, 'out>(
                    &'me self,
                    event: Event,
                    ctx: &'ctx Self::Context,
                ) -> Self::TransformedStream<'out>
                where
                    'me: 'out,
                    'ctx: 'out,
                {
                    #[allow(unused_imports)]
                    use ::arcana::es::adapter::transformer::specialization::{
                        TransformedBySkipAdapter as _,
                        TransformedByAdapter as _,
                        TransformedByFrom as _,
                        TransformedByFromInitial as _,
                        TransformedByFromUpcast as _,
                        TransformedByFromInitialUpcast as _,
                        TransformedByEmpty as _,
                        Wrap,
                    };

                    if ::std::option::Option::is_some(
                        &::arcana::es::event::codegen::Get::<{
                                0usize % <Event as ::arcana::es::event::
                                    codegen::EnumSize>::SIZE
                            }>::get(&event)
                    ) {
                        let event =
                            ::arcana::es::event::codegen::Get::<{
                                0usize % <Event as ::arcana::es::event::
                                    codegen::EnumSize>::SIZE
                            }>::unwrap(event);
                        let check = || {
                            struct AssertImplAnyFallback;

                            struct ActualAssertImplAnyToken;
                            trait AssertImplAnyToken {}
                            impl AssertImplAnyToken for
                                ActualAssertImplAnyToken {}

                            fn assert_impl_any_token<T>(_: T)
                            where T: AssertImplAnyToken {}

                            let previous = AssertImplAnyFallback;

                            let previous = {
                                struct Wrapper<T, N>(
                                    ::std::marker::PhantomData<T>,
                                    N,
                                );

                                impl<T, N> ::std::ops::Deref for Wrapper<T, N> {
                                    type Target = N;

                                    fn deref(&self) -> &Self::Target {
                                        &self.1
                                    }
                                }

                                impl<T, N> Wrapper<T, N> {
                                    fn new(_: &T, right: N) -> Wrapper<T, N> {
                                        Self(::std::marker::PhantomData, right)
                                    }
                                }

                                impl<T, N> Wrapper<T, N>
                                where
                                    T: ::arcana::es::event::Versioned,
                                {
                                    fn _static_assertions_impl_any(
                                        &self,
                                    ) -> ActualAssertImplAnyToken {
                                        ActualAssertImplAnyToken
                                    }
                                }

                                Wrapper::new(&event, previous)
                            };

                            let previous = {
                                struct Wrapper<T, N>(
                                    ::std::marker::PhantomData<T>,
                                    N,
                                );

                                impl<T, N> ::std::ops::Deref for Wrapper<T, N> {
                                    type Target = N;

                                    fn deref(&self) -> &Self::Target {
                                        &self.1
                                    }
                                }

                                impl<T, N> Wrapper<T, N> {
                                    fn new(_: &T, right: N) -> Wrapper<T, N> {
                                        Self(::std::marker::PhantomData, right)
                                    }
                                }

                                impl<T, N> Wrapper<T, N>
                                where
                                    T: ::arcana::es::adapter::TransformedBy<
                                        Adapter
                                    >,
                                {
                                    fn _static_assertions_impl_any(
                                        &self,
                                    ) -> ActualAssertImplAnyToken {
                                        ActualAssertImplAnyToken
                                    }
                                }

                                Wrapper::new(&event, previous)
                            };

                            assert_impl_any_token(
                                previous._static_assertions_impl_any(),
                            );

                            event
                        };
                        let event = check();

                        ::std::boxed::Box::pin(
                            (&&&&&&&Wrap::<&Adapter, _, IntoEvent>(
                                self,
                                &event,
                                ::std::marker::PhantomData,
                            ))
                                .get_tag()
                                .transform_event(self, event, ctx),
                            )
                    } else if ::std::option::Option::is_some(
                        &::arcana::es::event::codegen::Get::<{
                            1usize % <Event as ::arcana::es::event::
                            codegen::EnumSize>::SIZE
                        }>::get(&event)
                    ) {
                        let event =
                        ::arcana::es::event::codegen::Get::<{
                            1usize % <Event as ::arcana::es::event::codegen::
                                EnumSize>::SIZE
                        }>::unwrap(event);
                        let check = || {
                            struct AssertImplAnyFallback;

                            struct ActualAssertImplAnyToken;
                            trait AssertImplAnyToken {}
                            impl AssertImplAnyToken for
                                ActualAssertImplAnyToken {}

                            fn assert_impl_any_token<T>(_: T)
                            where T: AssertImplAnyToken {}

                            let previous = AssertImplAnyFallback;

                            let previous = {
                                struct Wrapper<T, N>(
                                    ::std::marker::PhantomData<T>,
                                    N,
                                );

                                impl<T, N> ::std::ops::Deref for Wrapper<T, N> {
                                    type Target = N;

                                    fn deref(&self) -> &Self::Target {
                                        &self.1
                                    }
                                }

                                impl<T, N> Wrapper<T, N> {
                                    fn new(_: &T, right: N) -> Wrapper<T, N> {
                                        Self(::std::marker::PhantomData, right)
                                    }
                                }

                                impl<T, N> Wrapper<T, N>
                                where
                                    T: ::arcana::es::event::Versioned,
                                {
                                    fn _static_assertions_impl_any(
                                        &self,
                                    ) -> ActualAssertImplAnyToken {
                                        ActualAssertImplAnyToken
                                    }
                                }

                                Wrapper::new(&event, previous)
                            };

                            let previous = {
                                struct Wrapper<T, N>(
                                    ::std::marker::PhantomData<T>,
                                    N,
                                );

                                impl<T, N> ::std::ops::Deref for Wrapper<T, N> {
                                    type Target = N;

                                    fn deref(&self) -> &Self::Target {
                                        &self.1
                                    }
                                }

                                impl<T, N> Wrapper<T, N> {
                                    fn new(_: &T, right: N) -> Wrapper<T, N> {
                                        Self(::std::marker::PhantomData, right)
                                    }
                                }

                                impl<T, N> Wrapper<T, N>
                                where
                                    T: ::arcana::es::adapter::TransformedBy<
                                        Adapter
                                    >,
                                {
                                    fn _static_assertions_impl_any(
                                        &self,
                                    ) -> ActualAssertImplAnyToken {
                                        ActualAssertImplAnyToken
                                    }
                                }

                                Wrapper::new(&event, previous)
                            };

                            assert_impl_any_token(
                                previous._static_assertions_impl_any(),
                            );

                            event
                        };
                        let event = check();

                        ::std::boxed::Box::pin(
                            (&&&&&&&Wrap::<&Adapter, _, IntoEvent>(
                                self,
                                &event,
                                ::std::marker::PhantomData,
                            ))
                                .get_tag()
                                .transform_event(self, event, ctx),
                            )
                        } else {
                            unreachable!()
                        }
                    }
                }
        };

        assert_eq!(
            super::derive(input).unwrap().to_string(),
            output.to_string(),
        );
    }

    #[allow(clippy::too_many_lines)]
    #[test]
    fn derives_multiple_impls() {
        let shorter_input = parse_quote! {
            #[event(
                transformer(
                    events(FirstEvent, SecondEvent),
                    into = IntoEvent,
                    context = dyn Any,
                    error = Infallible,
                    max_number_of_variants = 1,
                ),
            )]
            struct Adapter;
        };

        let longer_input = parse_quote! {
            #[event(
                transformer(
                    event = FirstEvent,
                    into = IntoEvent,
                    context = dyn Any,
                    error = Infallible,
                    max_number_of_variants = 1,
                ),
                transformer(
                    event = SecondEvent,
                    into = IntoEvent,
                    context = dyn Any,
                    error = Infallible,
                    max_number_of_variants = 1,
                ),
            )]
            struct Adapter;
        };

        let output = quote! {
            ::arcana::es::adapter::transformer::too_many_variants_in_enum!(
                <FirstEvent as ::arcana::es::event::codegen::EnumSize>::SIZE <=
                    1usize
            );

            #[automatically_derived]
            impl ::arcana::es::adapter::Transformer<FirstEvent> for Adapter {
                type Context = dyn Any;
                type Error = Infallible;
                type Transformed = IntoEvent;
                #[allow(clippy::type_complexity)]
                type TransformedStream<'out> =
                    ::std::pin::Pin<
                        ::std::boxed::Box<
                            dyn ::arcana::es::event::codegen::futures::Stream<
                                Item = ::std::result::Result<
                                    Self::Transformed,
                                    Self::Error,
                                >
                            > + 'out
                        >
                    >;

                #[allow(clippy::modulo_one)]
                fn transform<'me, 'ctx, 'out>(
                    &'me self,
                    event: FirstEvent,
                    ctx: &'ctx Self::Context,
                ) -> Self::TransformedStream<'out>
                where
                    'me: 'out,
                    'ctx: 'out,
                {
                    #[allow(unused_imports)]
                    use ::arcana::es::adapter::transformer::specialization::{
                        TransformedBySkipAdapter as _,
                        TransformedByAdapter as _,
                        TransformedByFrom as _,
                        TransformedByFromInitial as _,
                        TransformedByFromUpcast as _,
                        TransformedByFromInitialUpcast as _,
                        TransformedByEmpty as _,
                        Wrap,
                    };

                    if ::std::option::Option::is_some(
                        &::arcana::es::event::codegen::Get::<{
                                0usize % <FirstEvent as ::arcana::es::event::
                                    codegen::EnumSize>::SIZE
                            }>::get(&event)
                    ) {
                        let event =
                            ::arcana::es::event::codegen::Get::<{
                                0usize % <FirstEvent as ::arcana::es::event::
                                    codegen::EnumSize>::SIZE
                            }>::unwrap(event);
                        let check = || {
                            struct AssertImplAnyFallback;

                            struct ActualAssertImplAnyToken;
                            trait AssertImplAnyToken {}
                            impl AssertImplAnyToken for
                                ActualAssertImplAnyToken {}

                            fn assert_impl_any_token<T>(_: T)
                            where T: AssertImplAnyToken {}

                            let previous = AssertImplAnyFallback;

                            let previous = {
                                struct Wrapper<T, N>(
                                    ::std::marker::PhantomData<T>,
                                    N,
                                );

                                impl<T, N> ::std::ops::Deref for Wrapper<T, N> {
                                    type Target = N;

                                    fn deref(&self) -> &Self::Target {
                                        &self.1
                                    }
                                }

                                impl<T, N> Wrapper<T, N> {
                                    fn new(_: &T, right: N) -> Wrapper<T, N> {
                                        Self(::std::marker::PhantomData, right)
                                    }
                                }

                                impl<T, N> Wrapper<T, N>
                                where
                                    T: ::arcana::es::event::Versioned,
                                {
                                    fn _static_assertions_impl_any(
                                        &self,
                                    ) -> ActualAssertImplAnyToken {
                                        ActualAssertImplAnyToken
                                    }
                                }

                                Wrapper::new(&event, previous)
                            };

                            let previous = {
                                struct Wrapper<T, N>(
                                    ::std::marker::PhantomData<T>,
                                    N,
                                );

                                impl<T, N> ::std::ops::Deref for Wrapper<T, N> {
                                    type Target = N;

                                    fn deref(&self) -> &Self::Target {
                                        &self.1
                                    }
                                }

                                impl<T, N> Wrapper<T, N> {
                                    fn new(_: &T, right: N) -> Wrapper<T, N> {
                                        Self(::std::marker::PhantomData, right)
                                    }
                                }

                                impl<T, N> Wrapper<T, N>
                                where
                                    T: ::arcana::es::adapter::TransformedBy<
                                        Adapter
                                    >,
                                {
                                    fn _static_assertions_impl_any(
                                        &self,
                                    ) -> ActualAssertImplAnyToken {
                                        ActualAssertImplAnyToken
                                    }
                                }

                                Wrapper::new(&event, previous)
                            };

                            assert_impl_any_token(
                                previous._static_assertions_impl_any(),
                            );

                            event
                        };
                        let event = check();

                        ::std::boxed::Box::pin(
                            (&&&&&&&Wrap::<&Adapter, _, IntoEvent>(
                                self,
                                &event,
                                ::std::marker::PhantomData,
                            ))
                                .get_tag()
                                .transform_event(self, event, ctx),
                            )
                    } else {
                        unreachable!()
                    }
                }
            }

            ::arcana::es::adapter::transformer::too_many_variants_in_enum!(
                <SecondEvent as ::arcana::es::event::codegen::EnumSize>::SIZE <=
                    1usize
            );

            #[automatically_derived]
            impl ::arcana::es::adapter::Transformer<SecondEvent> for Adapter {
                type Context = dyn Any;
                type Error = Infallible;
                type Transformed = IntoEvent;
                #[allow(clippy::type_complexity)]
                type TransformedStream<'out> =
                    ::std::pin::Pin<
                        ::std::boxed::Box<
                            dyn ::arcana::es::event::codegen::futures::Stream<
                                Item = ::std::result::Result<
                                    Self::Transformed,
                                    Self::Error,
                                >
                            > + 'out
                        >
                    >;

                #[allow(clippy::modulo_one)]
                fn transform<'me, 'ctx, 'out>(
                    &'me self,
                    event: SecondEvent,
                    ctx: &'ctx Self::Context,
                ) -> Self::TransformedStream<'out>
                where
                    'me: 'out,
                    'ctx: 'out,
                {
                    #[allow(unused_imports)]
                    use ::arcana::es::adapter::transformer::specialization::{
                        TransformedBySkipAdapter as _,
                        TransformedByAdapter as _,
                        TransformedByFrom as _,
                        TransformedByFromInitial as _,
                        TransformedByFromUpcast as _,
                        TransformedByFromInitialUpcast as _,
                        TransformedByEmpty as _,
                        Wrap,
                    };

                    if ::std::option::Option::is_some(
                        &::arcana::es::event::codegen::Get::<{
                                0usize % <SecondEvent as ::arcana::es::event::
                                    codegen::EnumSize>::SIZE
                            }>::get(&event)
                    ) {
                        let event =
                            ::arcana::es::event::codegen::Get::<{
                                0usize % <SecondEvent as ::arcana::es::event::
                                    codegen::EnumSize>::SIZE
                            }>::unwrap(event);
                        let check = || {
                            struct AssertImplAnyFallback;

                            struct ActualAssertImplAnyToken;
                            trait AssertImplAnyToken {}
                            impl AssertImplAnyToken for
                                ActualAssertImplAnyToken {}

                            fn assert_impl_any_token<T>(_: T)
                            where T: AssertImplAnyToken {}

                            let previous = AssertImplAnyFallback;

                            let previous = {
                                struct Wrapper<T, N>(
                                    ::std::marker::PhantomData<T>,
                                    N,
                                );

                                impl<T, N> ::std::ops::Deref for Wrapper<T, N> {
                                    type Target = N;

                                    fn deref(&self) -> &Self::Target {
                                        &self.1
                                    }
                                }

                                impl<T, N> Wrapper<T, N> {
                                    fn new(_: &T, right: N) -> Wrapper<T, N> {
                                        Self(::std::marker::PhantomData, right)
                                    }
                                }

                                impl<T, N> Wrapper<T, N>
                                where
                                    T: ::arcana::es::event::Versioned,
                                {
                                    fn _static_assertions_impl_any(
                                        &self,
                                    ) -> ActualAssertImplAnyToken {
                                        ActualAssertImplAnyToken
                                    }
                                }

                                Wrapper::new(&event, previous)
                            };

                            let previous = {
                                struct Wrapper<T, N>(
                                    ::std::marker::PhantomData<T>,
                                    N,
                                );

                                impl<T, N> ::std::ops::Deref for Wrapper<T, N> {
                                    type Target = N;

                                    fn deref(&self) -> &Self::Target {
                                        &self.1
                                    }
                                }

                                impl<T, N> Wrapper<T, N> {
                                    fn new(_: &T, right: N) -> Wrapper<T, N> {
                                        Self(::std::marker::PhantomData, right)
                                    }
                                }

                                impl<T, N> Wrapper<T, N>
                                where
                                    T: ::arcana::es::adapter::TransformedBy<
                                        Adapter
                                    >,
                                {
                                    fn _static_assertions_impl_any(
                                        &self,
                                    ) -> ActualAssertImplAnyToken {
                                        ActualAssertImplAnyToken
                                    }
                                }

                                Wrapper::new(&event, previous)
                            };

                            assert_impl_any_token(
                                previous._static_assertions_impl_any(),
                            );

                            event
                        };
                        let event = check();

                        ::std::boxed::Box::pin(
                            (&&&&&&&Wrap::<&Adapter, _, IntoEvent>(
                                self,
                                &event,
                                ::std::marker::PhantomData,
                            ))
                                .get_tag()
                                .transform_event(self, event, ctx),
                            )
                    } else {
                        unreachable!()
                    }
                }
            }
        };

        let shorter = super::derive(shorter_input).unwrap().to_string();
        let longer = super::derive(longer_input).unwrap().to_string();

        assert_eq!(shorter, longer);
        assert_eq!(shorter, output.to_string());
    }

    #[test]
    fn errors_on_without_event_attribute() {
        let input = parse_quote! {
            #[event(
                transformer(
                    transformed = IntoEvent,
                    context = dyn Any,
                    error = Infallible,
                ),
            )]
            struct Adapter;
        };

        let err = super::derive(input).unwrap_err();

        assert_eq!(
            err.to_string(),
            "expected at least 1 `event` or `from` attribute",
        );
    }

    #[test]
    fn errors_on_without_transformed_attribute() {
        let input = parse_quote! {
            #[event(
                transformer(
                    event = Event,
                    context = dyn Any,
                    error = Infallible,
                ),
            )]
            enum Event {
                Event1(Event1),
                Event2(Event2),
            }
        };

        let err = super::derive(input).unwrap_err();

        assert_eq!(
            err.to_string(),
            "either `into` or `transformed` argument of \
             `#[event(transformer)]` attribute is expected to be present, \
             but is absent",
        );
    }

    #[test]
    fn errors_on_without_context_attribute() {
        let input = parse_quote! {
            #[event(
                transformer(
                    from = Event,
                    transformed = IntoEvent,
                    error = Infallible,
                ),
            )]
            struct Adapter;
        };

        let err = super::derive(input).unwrap_err();

        assert_eq!(
            err.to_string(),
            "either `context` or `ctx` argument of \
             `#[event(transformer)]` attribute is expected to be present, \
             but is absent",
        );
    }

    #[test]
    fn errors_on_without_error_attribute() {
        let input = parse_quote! {
            #[event(
                transformer(
                    from = Event,
                    into = IntoEvent,
                    ctx = dyn Any,
                ),
            )]
            struct Adapter;
        };

        let err = super::derive(input).unwrap_err();

        assert_eq!(
            err.to_string(),
            "either `err` or `error` argument of \
             `#[event(transformer)]` attribute is expected to be present, \
             but is absent",
        );
    }
}
