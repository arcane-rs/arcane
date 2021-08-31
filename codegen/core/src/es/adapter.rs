//! `#[derive(adapter::Transformer)]` macro implementation.

use std::convert::TryFrom;

use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;
use synthez::{ParseAttrs, Required, Spanning, ToTokens};

pub fn derive(input: TokenStream) -> syn::Result<TokenStream> {
    let input = syn::parse2::<syn::DeriveInput>(input)?;
    let definition = Definition::try_from(input)?;

    Ok(quote! { #definition })
}

/// Helper attributes of `#[derive(adapter::Transformer)]` macro.
#[derive(Debug, Default, ParseAttrs)]
pub struct Attrs {
    #[parse(nested)]
    pub transformer: Required<Spanning<TransformerAttrs>>,
}

#[derive(Debug, Default, ParseAttrs)]
pub struct TransformerAttrs {
    /// Type to derive [`adapter::Transformer`][0] on.
    ///
    /// [0]: arcana_core::es::adapter::Transformer
    #[parse(value)]
    pub adapter: Required<syn::TypePath>,

    /// [`adapter::Transformer::Transformed`][0] type.
    ///
    /// [0]: arcana_core::es::adapter::Transformer::Transformed
    #[parse(value, alias = into)]
    pub transformed: Required<syn::TypePath>,

    /// [`adapter::Transformer::Context`][0] type.
    ///
    /// [0]: arcana_core::es::adapter::Transformer::Context
    #[parse(value, alias = ctx)]
    pub context: Required<syn::Type>,

    /// [`adapter::Transformer::Error`][0] type.
    ///
    /// [0]: arcana_core::es::adapter::Transformer::Error
    #[parse(value, alias = err)]
    pub error: Required<syn::TypePath>,
}

#[derive(Debug, ToTokens)]
#[to_tokens(append(derive_transformer, from_unknown))]
pub struct Definition {
    pub ident: syn::Ident,
    pub generics: syn::Generics,
    pub variants: Vec<syn::Variant>,
    pub adapter: syn::TypePath,
    pub transformed: syn::TypePath,
    pub context: syn::Type,
    pub error: syn::TypePath,
}

impl TryFrom<syn::DeriveInput> for Definition {
    type Error = syn::Error;

    fn try_from(input: syn::DeriveInput) -> syn::Result<Self> {
        let attrs = Attrs::parse_attrs("event", &input)?;
        let TransformerAttrs {
            adapter,
            transformed,
            context,
            error,
        } = attrs.transformer.into_inner().into_inner();

        let data = if let syn::Data::Enum(data) = input.data {
            data
        } else {
            return Err(syn::Error::new(input.span(), "expected enum only"));
        };

        Ok(Self {
            ident: input.ident,
            generics: input.generics,
            variants: data.variants.into_iter().collect(),
            adapter: adapter.into_inner(),
            transformed: transformed.into_inner(),
            context: context.into_inner(),
            error: error.into_inner(),
        })
    }
}

impl Definition {
    fn derive_transformer(&self) -> TokenStream {
        let event = &self.ident;
        let adapter = &self.adapter;
        let context = &self.context;
        let error = &self.error;
        let transformed = &self.transformed;
        let inner_match = self.inner_match();
        let transformed_stream = self.transformed_stream();

        quote! {
            impl ::arcana::es::adapter::Transformer<#event> for #adapter {
                type Context = #context;
                type Error = #error;
                type Transformed = #transformed;
                type TransformedStream<'me, 'ctx> = #transformed_stream;

                fn transform<'me, 'ctx>(
                    &'me self,
                    event: #event,
                    context: &'ctx <Self as
                        ::arcana::es::adapter::Transformer<#event>>::Context,
                ) -> <Self as ::arcana::es::adapter::Transformer<#event>>::
                        TransformedStream<'me, 'ctx>
                {
                    use ::arcana::codegen::futures::StreamExt as _;

                    fn transform_result<Ok, Err, IntoOk, IntoErr>(
                        res: Result<Ok, Err>,
                    ) -> Result<IntoOk, IntoErr>
                    where
                        IntoOk: From<Ok>,
                        IntoErr: From<Err>,
                    {
                        ::std::result::Result::map_err(
                                ::std::result::Result::map(
                                    res,
                                    ::std::convert::Into::into,
                                ),
                                ::std::convert::Into::into,
                            )
                    }

                    match event {
                        #inner_match
                    }
                }
            }
        }
    }

    fn transformed_stream(&self) -> TokenStream {
        let adapter = &self.adapter;
        let from = &self.ident;

        let stream = |ev: TokenStream| quote! {
            ::arcana::codegen::futures::stream::Map<
                <#adapter as ::arcana::es::adapter::Transformer<#ev>>::
                    TransformedStream<'me, 'ctx>,
                fn(
                    Result<
                        <#adapter as ::arcana::es::adapter::Transformer<#ev>>::
                            Transformed,
                        <#adapter as ::arcana::es::adapter::Transformer<#ev>>::
                            Error,
                    >,
                ) -> Result<
                    <#adapter as ::arcana::es::adapter::Transformer<#from>>::
                        Transformed,
                    <#adapter as ::arcana::es::adapter::Transformer<#from>>::
                        Error,
                >,
            >
        };

        let last_variant= &self
            .variants
            .last()
            .unwrap()
            .fields
            .iter()
            .next()
            .unwrap()
            .ty;
        let last_variant = stream(last_variant.into_token_stream());

        self
            .variants
            .iter()
            .map(|var| &var.fields.iter().next().unwrap().ty)
            .rev()
            .skip(1)
            .fold(last_variant, |ty, variant| {
                let variant = stream(variant.into_token_stream());
                quote! {
                    ::arcana::codegen::futures::future::Either<
                        #variant,
                        #ty,
                    >
                }
            })
    }

    fn inner_match(&self) -> TokenStream {
        let event = &self.ident;
        let adapter = &self.adapter;

        let variant = &self.variants.first().unwrap().ident;
        let variant_val = &self
            .variants
            .first()
            .unwrap()
            .fields
            .iter()
            .next()
            .unwrap()
            .ty;

        let matcher = |variant: TokenStream, variant_val: TokenStream, ext: TokenStream| {
            quote! {
                #event::#variant(event) => {
                    <#adapter as ::arcana::es::adapter::Transformer<
                        #variant_val
                    >>::transform(self, event, context)
                        .map(transform_result as fn(_) -> _)
                        #ext
                },
            }
        };

        if self.variants.len() == 1 {
            return matcher(variant.into_token_stream(), variant_val.into_token_stream(), quote! {});
        }

        self.variants
            .iter()
            .enumerate()
            .map(|(i, var)| {
                let variant = &var.ident;
                let variant_val = &var.fields.iter().next().unwrap().ty;

                let left_stream =
                    (i == self.variants.len() - 1).then(|| 0).unwrap_or(1);
                let convert = std::iter::repeat(quote! { .left_stream() })
                    .take(left_stream)
                    .chain(
                        std::iter::repeat(quote! { .right_stream() }).take(i),
                    )
                    .collect();
                matcher(variant.into_token_stream(), variant_val.into_token_stream(), convert)
            })
            .collect()
    }

    fn from_unknown(&self) -> TokenStream {
        let transformed = &self.transformed;
        quote! {
            impl From<::arcana::es::adapter::transformer::strategy::Unknown>
                for #transformed
            {
                fn from(
                    u: ::arcana::es::adapter::transformer::strategy::Unknown,
            ) -> Self {
                    match u {}
                }
            }
        }
    }
}
