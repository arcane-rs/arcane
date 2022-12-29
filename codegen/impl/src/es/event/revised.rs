//! `#[derive(event::Revised)]` macro implementation.

use std::num::NonZeroU16;

use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned as _;
use synthez::{ParseAttrs, Required, ToTokens};

/// Expands `#[derive(event::Revised)]` macro.
///
/// # Errors
///
/// - If `input` isn't a Rust struct definition;
/// - If failed to parse [`Attrs`].
pub fn derive(input: TokenStream) -> syn::Result<TokenStream> {
    let input = syn::parse2::<syn::DeriveInput>(input)?;
    let definition = Definition::try_from(input)?;

    Ok(quote! { #definition })
}

/// Helper attributes of `#[derive(event::Revised)]` macro.
#[derive(Debug, Default, ParseAttrs)]
pub struct Attrs {
    /// Value of [`event::Revised::NAME`][0] constant.
    ///
    /// [0]: arcane_core::es::event::Revised::NAME
    #[parse(value)]
    pub name: Required<syn::LitStr>,

    /// Value of [`event::Revised::REVISION`][0] constant.
    ///
    /// [0]: arcane_core::es::event::Revised::REVISION
    #[parse(value, alias = rev, validate = can_parse_as_non_zero_u16)]
    pub revision: Required<syn::LitInt>,
}

/// Checks whether the given `value` can be parsed as [`NonZeroU16`].
fn can_parse_as_non_zero_u16(value: &Required<syn::LitInt>) -> syn::Result<()> {
    syn::LitInt::base10_parse::<NonZeroU16>(value).map(drop)
}

/// Representation of a struct implementing [`event::Revised`][0], used for
/// code generation.
///
/// [0]: arcane_core::es::event::Revised
#[derive(Debug, ToTokens)]
#[to_tokens(append(impl_event_revised, gen_uniqueness_glue_code))]
pub struct Definition {
    /// [`syn::Ident`](struct@syn::Ident) of this structure's type.
    pub ident: syn::Ident,

    /// [`syn::Generics`] of this structure's type.
    pub generics: syn::Generics,

    /// Value of [`event::Revised::NAME`][0] constant in the generated code.
    ///
    /// [0]: arcane_core::es::event::Revised::NAME
    pub event_name: syn::LitStr,

    /// Value of [`event::Revised::REVISION`][0] constant in the generated
    /// code.
    ///
    /// [0]: arcane_core::es::event::Revised::REVISION
    pub event_revision: syn::LitInt,
}

impl TryFrom<syn::DeriveInput> for Definition {
    type Error = syn::Error;

    fn try_from(input: syn::DeriveInput) -> syn::Result<Self> {
        if !matches!(input.data, syn::Data::Struct(..)) {
            return Err(syn::Error::new(
                input.span(),
                "expected struct only, \
                 consider using `arcane::es::Event` for enums",
            ));
        }

        let attrs = Attrs::parse_attrs("event", &input)?;

        Ok(Self {
            ident: input.ident,
            generics: input.generics,
            event_name: attrs.name.into_inner(),
            event_revision: attrs.revision.into_inner(),
        })
    }
}

impl Definition {
    /// Generates code to derive [`event::Revised`][0] trait.
    ///
    /// [0]: arcane_core::es::event::Revised
    #[must_use]
    pub fn impl_event_revised(&self) -> TokenStream {
        let ty = &self.ident;
        let (impl_gens, ty_gens, where_clause) = self.generics.split_for_impl();

        let (event_name, event_rev) = (&self.event_name, &self.event_revision);

        quote! {
            #[automatically_derived]
            impl #impl_gens ::arcane::es::event::Revised for #ty #ty_gens
                 #where_clause
            {
                const NAME: ::arcane::es::event::Name = #event_name;

                // SAFETY: Safe, as checked by proc macro in compile time.
                const REVISION: ::arcane::es::event::Revision = unsafe {
                    ::arcane::es::event::Revision::new_unchecked(#event_rev)
                };
            }
        }
    }

    /// Generates hidden machinery code used to statically check uniqueness of
    /// [`Event::name`] and [`Event::revision`].
    ///
    /// [`Event::name`]: arcane_core::es::Event::name
    /// [`Event::revision`]: arcane_core::es::Event::revision
    #[must_use]
    pub fn gen_uniqueness_glue_code(&self) -> TokenStream {
        let ty = &self.ident;
        let (impl_gens, ty_gens, where_clause) = self.generics.split_for_impl();

        // TODO: Replace `::std::concat!(...)` with `TypeId::of()` once it gets
        //       `const`ified.
        //       https://github.com/rust-lang/rust/issues/77125
        quote! {
            #[automatically_derived]
            #[doc(hidden)]
            impl #impl_gens ::arcane::es::event::codegen::Meta for #ty #ty_gens
                #where_clause
            {
                #[doc(hidden)]
                const META: &'static [(&'static str, &'static str, u16)] = &[(
                    ::std::concat!(
                        ::std::file!(),
                        "_",
                        ::std::line!(),
                        "_",
                        ::std::column!(),
                    ),
                    <Self as ::arcane::es::event::Revised>::NAME,
                    <Self as ::arcane::es::event::Revised>::REVISION.get()
                )];
            }
        }
    }
}

#[cfg(test)]
mod spec {
    use quote::quote;
    use syn::parse_quote;

    #[test]
    fn derives_struct_impl() {
        let input = parse_quote! {
            #[event(name = "event", revision = 1)]
            struct Event;
        };

        let output = quote! {
            #[automatically_derived]
            impl ::arcane::es::event::Revised for Event {
                const NAME: ::arcane::es::event::Name = "event";

                // SAFETY: Safe, as checked by proc macro in compile time.
                const REVISION: ::arcane::es::event::Revision = unsafe {
                    ::arcane::es::event::Revision::new_unchecked(1)
                };
            }

            #[automatically_derived]
            #[doc(hidden)]
            impl ::arcane::es::event::codegen::Meta for Event {
                #[doc(hidden)]
                const META: &'static [(&'static str, &'static str, u16)] = &[(
                    ::std::concat!(
                        ::std::file!(),
                        "_",
                        ::std::line!(),
                        "_",
                        ::std::column!(),
                    ),
                    <Self as ::arcane::es::event::Revised>::NAME,
                    <Self as ::arcane::es::event::Revised>::REVISION.get()
                )];
            }
        };

        assert_eq!(
            super::derive(input).unwrap().to_string(),
            output.to_string(),
        );
    }

    #[test]
    fn name_arg_is_required() {
        let input = parse_quote! {
            #[event(rev = 1)]
            struct Event;
        };

        let err = super::derive(input).unwrap_err();

        assert_eq!(
            err.to_string(),
            "`name` argument of `#[event]` attribute is expected to be \
             present, but is absent",
        );
    }

    #[test]
    fn revision_arg_is_required() {
        let input = parse_quote! {
            #[event(name = "event")]
            struct Event;
        };

        let err = super::derive(input).unwrap_err();

        assert_eq!(
            err.to_string(),
            "either `rev` or `revision` argument of `#[event]` attribute is \
             expected to be present, but is absent",
        );
    }

    #[test]
    fn errors_on_negative_revision() {
        let input = parse_quote! {
            #[event(name = "event", rev = -1)]
            struct Event;
        };

        let err = super::derive(input).unwrap_err();

        assert_eq!(err.to_string(), "invalid digit found in string");
    }

    #[test]
    fn errors_on_zero_revision() {
        let input = parse_quote! {
            #[event(name = "event", revision = 0)]
            struct Event;
        };

        let err = super::derive(input).unwrap_err();

        assert_eq!(err.to_string(), "number would be zero for non-zero type",);
    }

    #[test]
    fn errors_on_u16_overflowed_revision() {
        let input = parse_quote! {
            #[event(name = "event", revision = 4294967295)]
            struct Event;
        };

        let err = super::derive(input).unwrap_err();

        assert_eq!(err.to_string(), "number too large to fit in target type",);
    }

    #[test]
    fn errors_on_enum() {
        let input = parse_quote! {
            #[event(name = "event", revision = 1)]
            enum Event {
                Event1(Event1),
            }
        };

        let err = super::derive(input).unwrap_err();

        assert_eq!(
            err.to_string(),
            "expected struct only, \
             consider using `arcane::es::Event` for enums",
        );
    }
}
