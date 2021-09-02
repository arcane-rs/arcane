//! `#[derive(adapter::Transformer)]` macro implementation.

use std::{convert::TryFrom, iter};

use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;
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
    /// Type to derive [`Transformer`][0] on.
    ///
    /// [0]: arcana_core::es::adapter::Transformer
    #[parse(value)]
    pub adapter: Required<syn::Type>,

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
}

impl From<InnerAttrs> for ImplDefinition {
    fn from(attrs: InnerAttrs) -> Self {
        let InnerAttrs {
            adapter,
            transformed,
            context,
            error,
        } = attrs;
        Self {
            adapter: adapter.into_inner(),
            transformed: transformed.into_inner(),
            context: context.into_inner(),
            error: error.into_inner(),
        }
    }
}

// TODO: add PartialEq impls in synthez
impl PartialEq for InnerAttrs {
    fn eq(&self, other: &Self) -> bool {
        *self.adapter == *other.adapter
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
    /// Generic parameter of the [`Transformer`][0].
    ///
    /// [0]: arcana_core::es::adapter::Transformer
    pub event: syn::Ident,

    /// [`syn::Generics`] of this enum's type.
    pub generics: syn::Generics,

    /// [`struct@syn::Ident`] and single [`syn::Type`] of every
    /// [`syn::FieldsUnnamed`] [`syn::Variant`].
    pub variants: Vec<(syn::Ident, syn::Type)>,

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
    /// Type to derive [`Transformer`][0] on.
    ///
    /// [0]: arcana_core::es::adapter::Transformer
    pub adapter: syn::Type,

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

        let data = if let syn::Data::Enum(data) = input.data {
            data
        } else {
            return Err(syn::Error::new(input.span(), "expected enum only"));
        };

        let variants = data
            .variants
            .into_iter()
            .map(Self::parse_variant)
            .collect::<syn::Result<Vec<_>>>()?;

        let transformers = attrs
            .transformer
            .into_iter()
            .map(|tr| tr.into_inner().into())
            .collect();

        Ok(Self {
            event: input.ident,
            generics: input.generics,
            variants,
            transformers,
        })
    }
}

impl Definition {
    /// Parses [`syn::Variant`], returning its [`syn::Ident`] and single inner
    /// [`syn::Field`].
    ///
    /// # Errors
    ///
    /// If [`syn::Variant`] doesn't have exactly one unnamed 1 [`syn::Field`].
    fn parse_variant(
        variant: syn::Variant,
    ) -> syn::Result<(syn::Ident, syn::Type)> {
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

        Ok((variant.ident, variant.fields.into_iter().next().unwrap().ty))
    }

    /// Generates code to derive [`Transformer`][0] trait.
    ///
    /// [0]: arcana_core::es::adapter::Transformer
    #[must_use]
    pub fn derive_transformer(&self) -> TokenStream {
        let event = &self.event;

        self.transformers.iter().map(|tr| {
            let ImplDefinition {
                adapter,
                transformed,
                context,
                error,
            } = tr;
            let inner_match = self.inner_match(adapter);
            let transformed_stream = self.transformed_stream(adapter);
            let (impl_gen, type_gen, where_clause) =
                self.generics.split_for_impl();

            quote! {
                #[automatically_derived]
                impl #impl_gen ::arcana::es::adapter::Transformer<
                    #event#type_gen
                > for #adapter #where_clause
                {
                    type Context = #context;
                    type Error = #error;
                    type Transformed = #transformed;
                    type TransformedStream<'me, 'ctx> = #transformed_stream;

                    fn transform<'me, 'ctx>(
                        &'me self,
                        __event: #event,
                        __context:
                            &'ctx <Self as ::arcana::es::adapter::
                                             Transformer<#event>>::Context,
                    ) -> <Self as ::arcana::es::adapter::Transformer<#event>>::
                                    TransformedStream<'me, 'ctx>
                    {
                        match __event {
                            #inner_match
                        }
                    }
                }
            }
        })
            .collect()
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
    pub fn transformed_stream(&self, adapter: &syn::Type) -> TokenStream {
        let from = &self.event;

        let transformed_stream = |event: &syn::Type| {
            quote! {
                ::arcana::codegen::futures::stream::Map<
                    <#adapter as ::arcana::es::adapter::Transformer<#event >>::
                                   TransformedStream<'me, 'ctx>,
                    fn(
                        ::std::result::Result<
                            <#adapter as ::arcana::es::adapter::
                                           Transformer<#event >>::Transformed,
                            <#adapter as ::arcana::es::adapter::
                                           Transformer<#event >>::Error,
                        >,
                    ) -> ::std::result::Result<
                        <#adapter as ::arcana::es::adapter::
                                       Transformer<#from>>::Transformed,
                        <#adapter as ::arcana::es::adapter::
                                       Transformer<#from>>::Error,
                    >,
                >
            }
        };

        self.variants
            .iter()
            .rev()
            .fold(None, |acc, (_, var_ty)| {
                let variant_stream = transformed_stream(var_ty);
                Some(
                    acc.map(|acc| {
                        quote! {
                            ::arcana::codegen::futures::future::Either<
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
    /// Generated code matches over every [`Event`]'s variant and makes it
    /// compatible with [`Self::transformed_stream()`] type with
    /// [`StreamExt::left_stream()`] and [`StreamExt::right_stream()`]
    /// combinators.
    ///
    /// [0]: arcana_core::es::adapter::Transformer::transform
    /// [`Event`]: trait@arcana_core::es::Event
    /// [`StreamExt::left_stream()`]: futures::StreamExt::left_stream()
    /// [`StreamExt::right_stream()`]: futures::StreamExt::right_stream()
    #[must_use]
    pub fn inner_match(&self, adapter: &syn::Type) -> TokenStream {
        let event = &self.event;

        self.variants
            .iter()
            .enumerate()
            .map(|(i, (variant_ident, variant_ty))| {
                let stream_map = quote! {
                    ::arcana::codegen::futures::StreamExt::map(
                        <#adapter as ::arcana::es::adapter::Transformer<
                            #variant_ty
                        >>::transform(self, __event, __context),
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
                    ::arcana::codegen::futures::StreamExt::right_stream
                };
                let left_stream = quote! {
                    ::arcana::codegen::futures::StreamExt::left_stream
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
}
