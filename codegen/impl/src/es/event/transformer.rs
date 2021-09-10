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
/// - If `input` isn't a Rust enum definition;
/// - If some enum variant is not a single-field tuple struct;
/// - If failed to parse [`Attrs`].
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
    /// TODO
    #[parse(value, alias = from)]
    pub event: Vec<syn::Type>,

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

    /// TODO
    #[parse(value, alias = max, validate = can_parse_as_non_zero_usize)]
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

impl InnerAttrs {
    fn into_impl_definition(
        self,
    ) -> impl Iterator<Item = syn::Result<ImplDefinition>> {
        let InnerAttrs {
            event,
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
                    .map_or(Ok(Definition::MAX_NUMBER_OF_VARIANTS), |max| {
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
        *self.event == *other.event
            && *self.transformed == *other.transformed
            && *self.context == *other.context
            && *self.error == *other.error
    }
}

/// Representation of a enum for implementing [`Transformer`][0], used for code
/// generation.
///
/// [0]: arcana_core::es::adapter::Transformer
#[derive(Debug, ToTokens)]
#[to_tokens(append(derive_transformer))]
pub struct Definition {
    /// TODO
    pub adapter: syn::Ident,

    /// [`syn::Generics`] of this enum's type.
    pub generics: syn::Generics,

    /// Definitions of structures to derive [`Transformer`][0] on.
    ///
    /// [0]: arcana_core::es::adapter::Transformer
    pub transformers: Vec<ImplDefinition>,
}

/// Representation of a struct implementing [`Transformer`][0], used for code
/// generation.
///
/// [0]: arcana_core::es::adapter::Transformer
#[derive(Debug)]
pub struct ImplDefinition {
    /// TODO
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

    /// TODO
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
    /// TODO
    pub const MAX_NUMBER_OF_VARIANTS: usize = 256;

    /// Generates code to derive [`Transformer`][0] trait.
    ///
    /// [0]: arcana_core::es::adapter::Transformer
    #[must_use]
    pub fn derive_transformer(&self) -> TokenStream {
        let adapter = &self.adapter;
        let (impl_gen, type_gen, where_clause) = self.generics.split_for_impl();
        let codegen_path = quote! { ::arcana::es::adapter::codegen };
        let specialization_path = quote! {
            ::arcana::es::adapter::transformer::specialization
        };
        let assert_fn = Self::assert_impl_any(
            &syn::Ident::new("event", Span::call_site()),
            [
                parse_quote! {  ::arcana::es::event::Versioned },
                parse_quote! {
                    ::arcana::es::adapter::TransformedBy<#adapter>
                },
            ],
        );

        self.transformers.iter().map(|tr| {
            let ImplDefinition {
                event,
                transformed,
                context,
                error,
                max_number_of_variants,
            } = tr;

            let max = *max_number_of_variants;
            let id = 0..max;
            let gets = quote! {
                #( if ::std::option::Option::is_some(
                    &#specialization_path::Get::<{
                        #id % <#event as #specialization_path::EnumSize>::SIZE
                    }>::get(&event)
                ) {
                    let event = #specialization_path::Get::<{
                        #id % <#event as #specialization_path::EnumSize>::SIZE
                    }>::unwrap(event);
                    let check = #assert_fn;
                    let event = check();

                    return ::std::boxed::Box::pin(
                        (&&&&&Wrap::<&#adapter, _, #transformed>(
                            self,
                            &event,
                            ::std::marker::PhantomData,
                        ))
                            .get_tag()
                            .transform_event(self, event, ctx),
                    );
                } else )*
                {
                    unreachable!()
                }
            };

            quote! {
                ::arcana::es::adapter::transformer::too_many_variants_in_enum!(
                    <#event as #specialization_path::EnumSize>::SIZE < #max
                );

                #[automatically_derived]
                impl #impl_gen ::arcana::es::adapter::Transformer<#event> for
                    #adapter#type_gen #where_clause
                {
                    type Context = #context;
                    type Error = #error;
                    type Transformed = #transformed;
                    #[allow(clippy::type_complexity)]
                    type TransformedStream<'me, 'ctx> =
                        ::std::pin::Pin<
                            ::std::boxed::Box<
                                dyn #codegen_path::futures::Stream<
                                    Item = ::std::result::Result<
                                        Self::Transformed,
                                        Self::Error,
                                    >
                                >
                            >
                        >;

                    #[allow(
                        clippy::modulo_one,
                        clippy::needless_borrow,
                        clippy::too_many_lines,
                        clippy::unused_self
                    )]
                    fn transform<'me, 'ctx>(
                        &'me self,
                        event: #event,
                        ctx: &'ctx Self::Context,
                    ) -> Self::TransformedStream<'me, 'ctx> {
                        #[allow(unused_imports)]
                        use #specialization_path::{
                            TransformedBySkipAdapter as _,
                            TransformedByAdapter as _,
                            TransformedByFrom as _,
                            TransformedByFromInitial as _,
                            TransformedByEmpty as _,
                            Wrap,
                        };

                        #gets
                    }
                }
            }
        })
            .collect()
    }

    /// TODO
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
    fn derives_enum_impl() {
        let input = parse_quote! {
            #[event(
                transformer(
                    adapter = Adapter,
                    into = IntoEvent,
                    context = dyn Any,
                    error = Infallible,
                ),
            )]
            enum Event {
                File(FileEvent),
                Chat(ChatEvent),
            }
        };

        let output = quote! {
            #[automatically_derived]
            impl ::arcana::es::adapter::Transformer<Event> for Adapter {
                type Context = dyn Any;
                type Error = Infallible;
                type Transformed = IntoEvent;
                type TransformedStream<'me, 'ctx> =
                    ::arcana::es::adapter::codegen::futures::future::Either<
                        ::arcana::es::adapter::codegen::futures::stream::Map<
                            <Adapter as ::arcana::es::adapter::Transformer<
                                FileEvent
                            >>::TransformedStream<'me, 'ctx>,
                            fn(
                                ::std::result::Result<
                                    <Adapter as ::arcana::es::adapter::
                                                  Transformer<FileEvent>>::
                                                  Transformed,
                                    <Adapter as ::arcana::es::adapter::
                                                  Transformer<FileEvent>>::
                                                  Error,
                                >,
                            ) -> ::std::result::Result<
                                <Adapter as ::arcana::es::adapter::
                                              Transformer<Event>>::
                                              Transformed,
                                <Adapter as ::arcana::es::adapter::
                                              Transformer<Event>>::Error,
                            >,
                        >,
                        ::arcana::es::adapter::codegen::futures::stream::Map<
                            <Adapter as ::arcana::es::adapter::Transformer<
                                ChatEvent
                            >>::TransformedStream<'me, 'ctx>,
                            fn(
                                ::std::result::Result<
                                    <Adapter as ::arcana::es::adapter::
                                                  Transformer<ChatEvent>>::
                                                  Transformed,
                                    <Adapter as ::arcana::es::adapter::
                                                  Transformer<ChatEvent>>::
                                                  Error,
                                >,
                            ) -> ::std::result::Result<
                                <Adapter as ::arcana::es::adapter::
                                               Transformer<Event>>::
                                               Transformed,
                                <Adapter as ::arcana::es::adapter::
                                              Transformer<Event>>::Error,
                            >,
                        >,
                    >;

                fn transform<'me, 'ctx>(
                    &'me self,
                    __event: Event,
                    __context:
                        &'ctx <Self as ::arcana::es::adapter::
                                         Transformer<Event>>::Context,
                ) -> <Self as ::arcana::es::adapter::Transformer<Event>>::
                                TransformedStream<'me, 'ctx>
                {
                    match __event {
                        Event::File(__event) => {
                            ::arcana::es::adapter::codegen::futures::StreamExt::
                                left_stream(
                                ::arcana::es::adapter::codegen::futures::
                                StreamExt::map(
                                    <Adapter as ::arcana::es::adapter::
                                                  Transformer<FileEvent>
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
                            ::arcana::es::adapter::codegen::futures::StreamExt::
                            right_stream(
                                ::arcana::es::adapter::codegen::futures::
                                StreamExt::map(
                                    <Adapter as ::arcana::es::adapter::
                                        Transformer<ChatEvent>
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
        };

        assert_eq!(
            super::derive(input).unwrap().to_string(),
            output.to_string(),
        );
    }

    #[test]
    fn errors_on_without_adapter_attribute() {
        let input = parse_quote! {
            #[event(
                transformer(
                    transformed = IntoEvent,
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
            "`adapter` argument of `#[event(transformer)]` attribute is \
             expected to be present, but is absent",
        );
    }

    #[test]
    fn errors_on_without_transformed_attribute() {
        let input = parse_quote! {
            #[event(
                transformer(
                    adapter = Adapter,
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
                    adapter = Adapter,
                    transformed = IntoEvent,
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
                    adapter = Adapter,
                    transformed = IntoEvent,
                    ctx = dyn Any,
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
            "either `err` or `error` argument of \
             `#[event(transformer)]` attribute is expected to be present, \
             but is absent",
        );
    }

    #[test]
    fn errors_on_multiple_fields_in_variant() {
        let input = parse_quote! {
            #[event(
                transformer(
                    adapter = Adapter,
                    into = IntoEvent,
                    context = dyn Any,
                    error = Infallible,
                ),
            )]
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
            #[event(
                transformer(
                    adapter = Adapter,
                    into = IntoEvent,
                    context = dyn Any,
                    error = Infallible,
                ),
            )]
            struct Event;
        };

        let err = super::derive(input).unwrap_err();

        assert_eq!(err.to_string(), "expected enum only");
    }

    #[test]
    fn errors_on_empty_enum() {
        let input = parse_quote! {
            #[event(
                transformer(
                    adapter = Adapter,
                    into = IntoEvent,
                    context = dyn Any,
                    error = Infallible,
                ),
            )]
            enum Event {}
        };

        let err = super::derive(input).unwrap_err();

        assert_eq!(err.to_string(), "enum must have at least one variant");
    }
}
