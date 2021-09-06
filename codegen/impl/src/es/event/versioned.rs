//! `#[derive(event::Versioned)]` macro implementation.

use std::{convert::TryFrom, num::NonZeroU16};

use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned as _;
use synthez::{ParseAttrs, Required, ToTokens};

/// Expands `#[derive(event::Versioned)]` macro.
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

/// Helper attributes of `#[derive(event::Versioned)]` macro.
#[derive(Debug, Default, ParseAttrs)]
pub struct Attrs {
    /// Value of [`event::Versioned::NAME`][0] constant.
    ///
    /// [0]: arcana_core::es::event::Versioned::NAME
    #[parse(value)]
    pub name: Required<syn::LitStr>,

    /// Value of [`event::Versioned::VERSION`][0] constant.
    ///
    /// [0]: arcana_core::es::event::Versioned::VERSION
    #[parse(value, alias = ver, validate = can_parse_as_non_zero_u16)]
    pub version: Required<syn::LitInt>,
}

/// Checks whether the given `value` can be parsed as [`NonZeroU16`].
fn can_parse_as_non_zero_u16(val: &Required<syn::LitInt>) -> syn::Result<()> {
    syn::LitInt::base10_parse::<NonZeroU16>(&**val).map(drop)
}

/// Representation of a struct implementing [`event::Versioned`][0], used for
/// code generation.
///
/// [0]: arcana_core::es::event::Versioned
#[derive(Debug, ToTokens)]
#[to_tokens(append(impl_event_versioned, gen_uniqueness_glue_code))]
pub struct Definition {
    /// [`syn::Ident`](struct@syn::Ident) of this structure's type.
    pub ident: syn::Ident,

    /// [`syn::Generics`] of this structure's type.
    pub generics: syn::Generics,

    /// Value of [`event::Versioned::NAME`][0] constant in the generated code.
    ///
    /// [0]: arcana_core::es::event::Versioned::NAME
    pub event_name: syn::LitStr,

    /// Value of [`event::Versioned::VERSION`][0] constant in the generated
    /// code.
    ///
    /// [0]: arcana_core::es::event::Versioned::VERSION
    pub event_version: syn::LitInt,
}

impl TryFrom<syn::DeriveInput> for Definition {
    type Error = syn::Error;

    fn try_from(input: syn::DeriveInput) -> syn::Result<Self> {
        if !matches!(input.data, syn::Data::Struct(..)) {
            return Err(syn::Error::new(
                input.span(),
                "expected struct only, \
                 consider using `arcana::es::Event` for enums",
            ));
        }

        let attrs = Attrs::parse_attrs("event", &input)?;

        Ok(Self {
            ident: input.ident,
            generics: input.generics,
            event_name: attrs.name.into_inner(),
            event_version: attrs.version.into_inner(),
        })
    }
}

impl Definition {
    /// Generates code to derive [`event::Versioned`][0] trait.
    ///
    /// [0]: arcana_core::es::event::Versioned
    #[must_use]
    pub fn impl_event_versioned(&self) -> TokenStream {
        let ty = &self.ident;
        let (impl_gens, ty_gens, where_clause) = self.generics.split_for_impl();

        let (event_name, event_ver) = (&self.event_name, &self.event_version);

        quote! {
            #[automatically_derived]
            impl #impl_gens ::arcana::es::event::Versioned for #ty#ty_gens
                 #where_clause
            {
                const NAME: ::arcana::es::event::Name = #event_name;

                // SAFETY: Safe, as checked by proc macro in compile time.
                const VERSION: ::arcana::es::event::Version = unsafe {
                    ::arcana::es::event::Version::new_unchecked(#event_ver)
                };
            }
        }
    }

    /// Generates hidden machinery code used to statically check uniqueness of
    /// [`Event::name`] and [`Event::version`].
    ///
    /// [`Event::name`]: arcana_core::es::Event::name
    /// [`Event::version`]: arcana_core::es::Event::version
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
            impl #impl_gens ::arcana::es::event::codegen::Versioned for
                 #ty#ty_gens #where_clause
            {
                #[doc(hidden)]
                const COUNT: usize = 1;
            }

            #[automatically_derived]
            #[doc(hidden)]
            impl #impl_gens #ty#ty_gens #where_clause {
                #[doc(hidden)]
                #[inline]
                pub const fn __arcana_events() ->
                    [(&'static str, &'static str, u16); 1]
                {
                    [(
                        ::std::concat!(
                            ::std::file!(),
                            "_",
                            ::std::line!(),
                            "_",
                            ::std::column!(),
                        ),
                        <Self as ::arcana::es::event::Versioned>::NAME,
                        <Self as ::arcana::es::event::Versioned>::VERSION.get(),
                    )]
                }
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
            #[event(name = "event", version = 1)]
            struct Event;
        };

        let output = quote! {
            #[automatically_derived]
            impl ::arcana::es::event::Versioned for Event {
                const NAME: ::arcana::es::event::Name = "event";

                // SAFETY: Safe, as checked by proc macro in compile time.
                const VERSION: ::arcana::es::event::Version = unsafe {
                    ::arcana::es::event::Version::new_unchecked(1)
                };
            }

            #[automatically_derived]
            #[doc(hidden)]
            impl ::arcana::es::event::codegen::Versioned for Event {
                #[doc(hidden)]
                const COUNT: usize = 1;
            }

            #[automatically_derived]
            #[doc(hidden)]
            impl Event {
                #[doc(hidden)]
                #[inline]
                pub const fn __arcana_events() ->
                    [(&'static str, &'static str, u16); 1]
                {
                    [(
                        ::std::concat!(
                            ::std::file!(),
                            "_",
                            ::std::line!(),
                            "_",
                            ::std::column!(),
                        ),
                        <Self as ::arcana::es::event::Versioned>::NAME,
                        <Self as ::arcana::es::event::Versioned>::VERSION.get(),
                    )]
                }
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
            #[event(ver = 1)]
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
    fn version_arg_is_required() {
        let input = parse_quote! {
            #[event(name = "event")]
            struct Event;
        };

        let err = super::derive(input).unwrap_err();

        assert_eq!(
            err.to_string(),
            "either `ver` or `version` argument of `#[event]` attribute is \
             expected to be present, but is absent",
        );
    }

    #[test]
    fn errors_on_negative_version() {
        let input = parse_quote! {
            #[event(name = "event", ver = -1)]
            struct Event;
        };

        let err = super::derive(input).unwrap_err();

        assert_eq!(err.to_string(), "invalid digit found in string");
    }

    #[test]
    fn errors_on_zero_version() {
        let input = parse_quote! {
            #[event(name = "event", version = 0)]
            struct Event;
        };

        let err = super::derive(input).unwrap_err();

        assert_eq!(err.to_string(), "number would be zero for non-zero type",);
    }

    #[test]
    fn errors_on_u16_overflowed_version() {
        let input = parse_quote! {
            #[event(name = "event", version = 4294967295)]
            struct Event;
        };

        let err = super::derive(input).unwrap_err();

        assert_eq!(err.to_string(), "number too large to fit in target type",);
    }

    #[test]
    fn errors_on_enum() {
        let input = parse_quote! {
            #[event(name = "event", version = 1)]
            enum Event {
                Event1(Event1),
            }
        };

        let err = super::derive(input).unwrap_err();

        assert_eq!(
            err.to_string(),
            "expected struct only, \
             consider using `arcana::es::Event` for enums",
        );
    }
}
