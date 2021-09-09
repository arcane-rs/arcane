//! `#[derive(adapter::Transformer)]` macro implementation.

use std::{convert::TryFrom, iter, num::NonZeroUsize};

use proc_macro2::{Span, TokenStream};
use quote::quote;
use std::ops::Deref;
use syn::{
    parse::{Parse, ParseStream},
    parse_quote,
    spanned::Spanned,
    token,
};
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
    pub event: Vec<EventAttrs>,

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
    #[parse(value, alias = ver, validate = can_parse_as_non_zero_usize)]
    pub number_of_events: Option<syn::LitInt>,
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
            number_of_events,
        } = self;

        event.into_iter().map(move |ev| {
            let event = ev
                .event
                .as_ref()
                .ok_or_else(|| syn::Error::new(Span::call_site(), "todo"))?;
            let num = ev
                .number_of_events
                .as_ref()
                .xor(number_of_events.as_ref())
                .ok_or_else(|| {
                    let span = if let (Some(l), Some(r)) = (
                        ev.number_of_events.as_ref(),
                        number_of_events.as_ref(),
                    ) {
                        l.span().join(r.span())
                    } else {
                        None
                    };

                    syn::Error::new(
                        span.unwrap_or_else(|| event.span()),
                        "exactly 1 'number_of_events' attribute expected",
                    )
                })?;

            Ok(ImplDefinition {
                event: event.clone(),
                transformed: transformed.deref().clone(),
                context: context.deref().clone(),
                error: error.deref().clone(),
                number_of_events: num.base10_parse()?,
            })
        })
    }
}

// TODO: add PartialEq impls in synthez
impl PartialEq for InnerAttrs {
    fn eq(&self, other: &Self) -> bool {
        *self.event == *other.event
            && *self.transformed == *other.transformed
            && *self.context == *other.context
            && *self.error == *other.error
            && self.number_of_events == other.number_of_events
    }
}

/// TODO
#[derive(Debug, Default)]
pub struct EventAttrs {
    event: Option<syn::Type>,
    number_of_events: Option<syn::LitInt>,
}

#[allow(dead_code)]
struct EventAttrsParse {
    event: syn::Type,
    comma: Option<syn::Token![,]>,
    ident: Option<syn::Ident>,
    equal: Option<syn::Token![=]>,
    number: Option<syn::LitInt>,
}

impl Parse for EventAttrs {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        input
            .peek(token::Paren)
            .then(|| {
                let content;
                syn::parenthesized!(content in input);
                Ok(content)
            })
            .transpose()?
            .map_or_else(
                || {
                    Ok(EventAttrs {
                        event: Some(input.parse::<syn::Type>()?),
                        number_of_events: None,
                    })
                },
                |input| {
                    let parsed = EventAttrsParse {
                        event: input.parse()?,
                        comma: input.parse()?,
                        ident: input.parse()?,
                        equal: input.parse()?,
                        number: input.parse()?,
                    };

                    if let Some(ident) = &parsed.ident {
                        if ident != "number_of_events" {
                            return Err(syn::Error::new(
                                parsed.ident.span(),
                                "expected number_of_events",
                            ));
                        }
                    }

                    Ok(EventAttrs {
                        event: Some(parsed.event),
                        number_of_events: parsed.number,
                    })
                },
            )
    }
}

impl ToTokens for EventAttrs {
    fn to_tokens(&self, tokens: &mut TokenStream) {
        let event = &self.event;
        let attr = if let Some(num) = &self.number_of_events {
            quote! { (#event, number_of_events = #num) }
        } else {
            quote! { #event }
        };

        attr.to_tokens(tokens);
    }
}

// TODO: add PartialEq impls in synthez
impl PartialEq for EventAttrs {
    fn eq(&self, other: &Self) -> bool {
        self.event == other.event
            && self.number_of_events == other.number_of_events
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
    pub number_of_events: NonZeroUsize,
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
    /// Generates code to derive [`Transformer`][0] trait.
    ///
    /// [0]: arcana_core::es::adapter::Transformer
    #[must_use]
    pub fn derive_transformer(&self) -> TokenStream {
        let adapter = &self.adapter;

        self.transformers.iter().map(|tr| {
            let ImplDefinition {
                event,
                transformed,
                context,
                error,
                number_of_events,
            } = tr;
            let inner_match = self.inner_match(transformed, *number_of_events);
            let (impl_gen, type_gen, where_clause) =
                self.generics.split_for_impl();

            let number_of_events = number_of_events.get();
            let codegen_path = quote! { ::arcana::es::adapter::codegen };
            let specialization_path = quote! {
                arcana::es::adapter::transformer::specialization
            };


            quote! {
                ::arcana::es::adapter::transformer::wrong_number_of_events!(
                    #number_of_events ==
                    <#event as #specialization_path::UnpackEnum>::TUPLE_SIZE,
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
                        clippy::too_many_lines,
                        clippy::unused_self,
                        clippy::needless_borrow
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

                        match #specialization_path::UnpackEnum::unpack(event) {
                            #inner_match
                        }
                    }
                }
            }
        })
            .collect()
    }

    /// TODO
    #[must_use]
    pub fn inner_match(
        &self,
        transformed: &syn::Type,
        number_of_events: NonZeroUsize,
    ) -> TokenStream {
        let number_of_events = number_of_events.get();
        let adapter = &self.adapter;

        let matches = (0..).take(number_of_events).map(|i| {
            let before_none = iter::repeat(quote! { None }).take(i);
            let after_none =
                iter::repeat(quote! { None }).take(number_of_events - i - 1);

            let assert_fn = Self::assert_impl_any(
                &syn::Ident::new("event", Span::call_site()),
                [
                    parse_quote! {  ::arcana::es::event::Versioned },
                    parse_quote! {
                        ::arcana::es::adapter::TransformedBy<#adapter>
                    },
                ],
            );

            quote! {
                ( #( #before_none, )* Some(event), #( #after_none ),* ) => {
                    let check = #assert_fn;
                    let event = check();

                    ::std::boxed::Box::pin(
                        (&&&&&Wrap::<&#adapter, _, #transformed>(
                            self,
                            &event,
                            ::std::marker::PhantomData,
                        ))
                            .get_tag()
                            .transform_event(self, event, ctx),
                    )
                }
            }
        });

        quote! {
            #( #matches )*
            _ => unreachable!(),
        }
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
