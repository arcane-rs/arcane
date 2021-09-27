//! `#[derive(adapter::Transformer)]` macro implementation.

use std::{collections::HashMap, convert::TryFrom};

use itertools::Itertools as _;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    parse::{Parse, ParseStream},
    parse_quote,
    punctuated::Punctuated,
};
use synthez::{ParseAttrs, ToTokens};

/// Expands `#[derive(Strategy)]` macro.
///
/// # Errors
///
/// - If failed to parse [`Attrs`].
pub fn derive(input: TokenStream) -> syn::Result<TokenStream> {
    let input = syn::parse2::<syn::DeriveInput>(input)?;
    let definition = Definition::try_from(input)?;

    Ok(quote! { #definition })
}

/// Helper attributes of `#[derive(Strategy)]` macro placed on an enum variant.
#[derive(Debug, Default, PartialEq)]
pub struct Attr {
    /// [`Strategies`][0] with corresponding [`VersionedEvent`][1]s.
    ///
    /// [0]: arcana_core::es::adapter::transformer::Strategy
    /// [1]: arcana_core::es::VersionedEvent
    pub strategies: HashMap<syn::Type, Vec<syn::Type>>,
}

impl Parse for Attr {
    fn parse(input: ParseStream<'_>) -> syn::Result<Self> {
        let parse_attr = |input: ParseStream<'_>| {
            let parenthesized = || {
                let content;
                let _ = syn::parenthesized!(content in input);
                Ok(content)
            };
            let events = || {
                parenthesized().map_or_else(
                    |_| input.parse().map(|ty| vec![ty]),
                    |par| {
                        par.parse_terminated::<_, syn::Token![,]>(
                            syn::Type::parse,
                        )
                        .map(|ty| ty.into_iter().collect::<Vec<_>>())
                    },
                )
            };

            let strategy = input.parse()?;
            let _ = input.parse::<syn::Token![=]>()?;
            let _ = input.parse::<syn::Token![>]>()?;

            Ok((strategy, events()?))
        };

        let strategies = input
            .parse_terminated::<_, syn::Token![,]>(parse_attr)?
            .into_iter()
            .collect::<HashMap<_, _>>();

        Ok(Self { strategies })
    }
}

impl ParseAttrs for Attr {
    fn try_merge(self, another: Self) -> syn::Result<Self> {
        Ok(Self {
            strategies: self
                .strategies
                .into_iter()
                .chain(another.strategies.into_iter())
                .collect(),
        })
    }
}

/// Representation of a enum for implementing [`Transformer`][0], used for code
/// generation.
///
/// [0]: arcana_core::es::adapter::Transformer
#[derive(Debug, ToTokens)]
#[to_tokens(append(impl_strategies))]
pub struct Definition {
    /// Generic parameter of the [`Transformer`][0].
    ///
    /// [0]: arcana_core::es::adapter::Transformer
    pub adapter: syn::Ident,

    /// [`syn::Generics`] of this enum's type.
    pub generics: syn::Generics,

    /// [`Strategies`][0] with corresponding [`VersionedEvent`][1]s.
    ///
    /// [0]: arcana_core::es::adapter::transformer::Strategy
    /// [1]: arcana_core::es::VersionedEvent
    pub strategies: HashMap<syn::Type, Vec<syn::Type>>,
}

impl TryFrom<syn::DeriveInput> for Definition {
    type Error = syn::Error;

    fn try_from(input: syn::DeriveInput) -> syn::Result<Self> {
        let attrs: Attr = Attr::parse_attrs("strategy", &input)?;

        Ok(Self {
            adapter: input.ident,
            generics: input.generics,
            strategies: attrs.strategies,
        })
    }
}

impl Definition {
    /// Generates code to derive [`Strategy`][0] traits.
    ///
    /// [0]: arcana_core::es::adapter::transformer::Strategy
    #[must_use]
    pub fn impl_strategies(&self) -> TokenStream {
        let transformed_and_err_bounds: Punctuated<
            syn::WherePredicate,
            syn::Token![,],
        > = parse_quote! {
            Self: ::arcana::es::adapter::WithError,
            <Self as ::arcana::es::adapter::WithError>::Transformed: 'static,
            <Self as ::arcana::es::adapter::WithError>::Error: 'static,
        };

        let mut generics = self.generics.clone();
        generics.params.push(parse_quote! { __Ctx });
        generics
            .make_where_clause()
            .predicates
            .extend(transformed_and_err_bounds);

        let (impl_gen, _, where_cl) = generics.split_for_impl();
        let (_, type_gen, _) = self.generics.split_for_impl();
        let adapter = &self.adapter;

        self.strategies
            .iter()
            .sorted_by_key(|(s, _)| s.to_token_stream().to_string())
            .map(|(strategy, ev)| {
                quote! {
                    #( impl#impl_gen ::arcana::es::adapter::transformer::
                        WithStrategy<#ev, __Ctx> for #adapter#type_gen #where_cl
                    {
                        type Strategy = #strategy;
                    } )*
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
