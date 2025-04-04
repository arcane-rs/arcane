//! `#[derive(Event)]` macro implementation for structs.

use std::num::NonZeroU16;

#[cfg(all(doc, feature = "doc"))]
use arcane_core::es::event;
use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned as _;
use synthez::{ParseAttrs, Required, ToTokens};

/// Attributes of `#[derive(Event)]` macro on structs.
#[derive(Debug, Default, ParseAttrs)]
pub struct Attrs {
    /// Value for the [`event::Static::NAME`] constant.
    #[parse(value)]
    pub name: Required<syn::LitStr>,

    /// Value fot the [`event::Concrete::REVISION`] constant.
    #[parse(value, alias = rev, validate = can_parse_as_non_zero_u16)]
    pub revision: Option<syn::LitInt>,
}

/// Checks whether the given `value` can be parsed as [`NonZeroU16`].
#[expect(clippy::ref_option, reason = "required by macro")]
fn can_parse_as_non_zero_u16(value: &Option<syn::LitInt>) -> syn::Result<()> {
    value.as_ref().map_or(Ok(()), |v| {
        syn::LitInt::base10_parse::<NonZeroU16>(v).map(drop)
    })
}

/// Representation of a struct implementing [`event::Static`] (and
/// [`event::Concrete`], optionally), used for the code generation.
// TODO: Provide a way to specify custom revision type.
#[derive(Debug, ToTokens)]
#[to_tokens(append(
    impl_event_static,
    impl_event_concrete,
    gen_uniqueness_assertion
))]
#[cfg_attr(
    feature = "reflect",
    to_tokens(append(impl_reflect_static, impl_reflect_concrete))
)]
pub struct Definition {
    /// [`syn::Ident`](struct@syn::Ident) of this structure's type.
    pub ident: syn::Ident,

    /// [`syn::Generics`] of this structure's type.
    pub generics: syn::Generics,

    /// Value of the [`event::Static::NAME`] constant in the generated code.
    pub event_name: syn::LitStr,

    /// Value of the [`event::Concrete::REVISION`] constant in the generated
    /// code.
    pub event_revision: Option<syn::LitInt>,
}

impl TryFrom<syn::DeriveInput> for Definition {
    type Error = syn::Error;

    fn try_from(input: syn::DeriveInput) -> syn::Result<Self> {
        if !matches!(input.data, syn::Data::Struct(..)) {
            return Err(syn::Error::new(
                input.span(),
                "only structs are allowed",
            ));
        }

        let attrs = Attrs::parse_attrs("event", &input)?;

        Ok(Self {
            ident: input.ident,
            generics: input.generics,
            event_name: attrs.name.into_inner(),
            event_revision: attrs.revision,
        })
    }
}

impl Definition {
    /// Generates code of an [`event::Static`] trait implementation.
    #[must_use]
    pub fn impl_event_static(&self) -> TokenStream {
        let ty = &self.ident;
        let (impl_gens, ty_gens, where_clause) = self.generics.split_for_impl();

        let event_name = &self.event_name;

        quote! {
            #[automatically_derived]
            impl #impl_gens ::arcane::es::event::Static for #ty #ty_gens
                 #where_clause
            {
                const NAME: ::arcane::es::event::Name = #event_name;
            }
        }
    }

    /// Generates code of an [`event::Concrete`] trait implementation.
    #[must_use]
    pub fn impl_event_concrete(&self) -> TokenStream {
        let Some(event_rev) = self.event_revision.as_ref() else {
            return TokenStream::new();
        };

        let ty = &self.ident;
        let (impl_gens, ty_gens, where_clause) = self.generics.split_for_impl();

        quote! {
            #[automatically_derived]
            impl #impl_gens ::arcane::es::event::Concrete for #ty #ty_gens
                 #where_clause
            {
                type Revision = ::arcane::es::event::Version;

                // SAFETY: Safe, as checked by proc macro in compile time.
                const REVISION: ::arcane::es::event::RevisionOf<Self> = unsafe {
                    ::arcane::es::event::Version::new_unchecked(#event_rev)
                };
            }
        }
    }

    #[cfg(feature = "reflect")]
    /// Generates code of an [`event::reflect::Static`] trait implementation.
    #[must_use]
    pub fn impl_reflect_static(&self) -> TokenStream {
        let ty = &self.ident;
        let (impl_gens, ty_gens, where_clause) = self.generics.split_for_impl();

        quote! {
            #[automatically_derived]
            impl #impl_gens ::arcane::es::event::reflect::Static
             for #ty #ty_gens #where_clause
            {
                const NAMES: &'static [::arcane::es::event::Name] =
                    &[<Self as ::arcane::es::event::Static>::NAME];
            }
        }
    }

    #[cfg(feature = "reflect")]
    /// Generates code of an [`event::reflect::Concrete`] trait implementation.
    #[must_use]
    pub fn impl_reflect_concrete(&self) -> TokenStream {
        if self.event_revision.is_none() {
            return TokenStream::new();
        }

        let ty = &self.ident;
        let (impl_gens, ty_gens, where_clause) = self.generics.split_for_impl();

        quote! {
            #[automatically_derived]
            impl #impl_gens ::arcane::es::event::reflect::Concrete
             for #ty #ty_gens #where_clause
            {
                // TODO: Replace with `::arcane::es::event::RevisionOf<Self>`
                //       once rust-lang/rust#57775 is resolved:
                //       https://github.com/rust-lang/rust/issues/57775
                const REVISIONS: &'static [::arcane::es::event::Version] = &[
                    <Self as ::arcane::es::event::Concrete>::REVISION
                ];
            }
        }
    }

    /// Generates non-public machinery code used to statically check whether
    /// [`Event::name`][0] and [`event::Revisable::revision`] pairs correspond
    /// to a single Rust type.
    ///
    /// [0]: event::Event::name
    #[must_use]
    pub fn gen_uniqueness_assertion(&self) -> TokenStream {
        let ty = &self.ident;
        let (impl_gens, ty_gens, where_clause) = self.generics.split_for_impl();

        let revision = self
            .event_revision
            .as_ref()
            .map(ToString::to_string)
            .unwrap_or_default();

        // TODO: Replace `::std::concat!(...)` with `TypeId::of()` once it gets
        //       `const`ified.
        //       https://github.com/rust-lang/rust/issues/77125
        quote! {
            #[automatically_derived]
            #[doc(hidden)]
            impl #impl_gens ::arcane::es::event::codegen::Reflect
             for #ty #ty_gens #where_clause
            {
                #[doc(hidden)]
                const META: &'static [
                    (&'static str, &'static str, &'static str)
                ] = &[(
                    ::std::concat!(
                        ::std::file!(),
                        "_",
                        ::std::line!(),
                        "_",
                        ::std::column!(),
                    ),
                    <Self as ::arcane::es::event::Static>::NAME,
                    #revision,
                )];
            }
        }
    }
}

#[cfg(test)]
mod spec {
    use proc_macro2::TokenStream;
    use quote::{ToTokens, quote};
    use syn::parse_quote;

    use super::Definition;

    /// Expands the `#[derive(Event)]` macro on the provided struct and returns
    /// the generated code.
    fn derive(input: TokenStream) -> syn::Result<TokenStream> {
        let input = syn::parse2::<syn::DeriveInput>(input)?;
        Ok(Definition::try_from(input)?.into_token_stream())
    }

    #[test]
    fn derives_struct_impl() {
        let input = parse_quote! {
            #[event(name = "event")]
            struct Event;
        };

        let mut output = quote! {
            #[automatically_derived]
            impl ::arcane::es::event::Static for Event {
                const NAME: ::arcane::es::event::Name = "event";
            }

            #[automatically_derived]
            #[doc(hidden)]
            impl ::arcane::es::event::codegen::Reflect for Event {
                #[doc(hidden)]
                const META: &'static [
                    (&'static str, &'static str, &'static str)
                ] = &[(
                    ::std::concat!(
                        ::std::file!(),
                        "_",
                        ::std::line!(),
                        "_",
                        ::std::column!(),
                    ),
                    <Self as ::arcane::es::event::Static>::NAME,
                    "",
                )];
            }
        };
        if cfg!(feature = "reflect") {
            output.extend([quote! {
                #[automatically_derived]
                impl ::arcane::es::event::reflect::Static for Event {
                    const NAMES: &'static [::arcane::es::event::Name] =
                        &[<Self as ::arcane::es::event::Static>::NAME];
                }
            }]);
        }

        assert_eq!(derive(input).unwrap().to_string(), output.to_string());
    }

    #[test]
    fn derives_struct_impl_with_revision() {
        let input = parse_quote! {
            #[event(name = "event", revision = 1)]
            struct Event;
        };

        let mut output = quote! {
            #[automatically_derived]
            impl ::arcane::es::event::Static for Event {
                const NAME: ::arcane::es::event::Name = "event";
            }

            #[automatically_derived]
            impl ::arcane::es::event::Concrete for Event {
                type Revision = ::arcane::es::event::Version;

                // SAFETY: Safe, as checked by proc macro in compile time.
                const REVISION: ::arcane::es::event::RevisionOf<Self> = unsafe {
                    ::arcane::es::event::Version::new_unchecked(1)
                };
            }

            #[automatically_derived]
            #[doc(hidden)]
            impl ::arcane::es::event::codegen::Reflect for Event {
                #[doc(hidden)]
                const META: &'static [
                    (&'static str, &'static str, &'static str)
                ] = &[(
                    ::std::concat!(
                        ::std::file!(),
                        "_",
                        ::std::line!(),
                        "_",
                        ::std::column!(),
                    ),
                    <Self as ::arcane::es::event::Static>::NAME,
                    "1",
                )];
            }
        };
        if cfg!(feature = "reflect") {
            output.extend([quote! {
                #[automatically_derived]
                impl ::arcane::es::event::reflect::Static for Event {
                    const NAMES: &'static [::arcane::es::event::Name] =
                        &[<Self as ::arcane::es::event::Static>::NAME];
                }

                #[automatically_derived]
                impl ::arcane::es::event::reflect::Concrete for Event {
                    const REVISIONS: &'static [::arcane::es::event::Version] =
                        &[<Self as ::arcane::es::event::Concrete>::REVISION];
                }
            }]);
        }

        assert_eq!(derive(input).unwrap().to_string(), output.to_string());
    }

    #[test]
    fn name_arg_is_required() {
        let input = parse_quote! {
            #[event(rev = 1)]
            struct Event;
        };

        let err = derive(input).unwrap_err();

        assert_eq!(
            err.to_string(),
            "`name` argument of `#[event]` attribute is expected to be \
             present, but is absent",
        );
    }

    #[test]
    fn errors_on_negative_revision() {
        let input = parse_quote! {
            #[event(name = "event", rev = -1)]
            struct Event;
        };

        let err = derive(input).unwrap_err();

        assert_eq!(err.to_string(), "invalid digit found in string");
    }

    #[test]
    fn errors_on_zero_revision() {
        let input = parse_quote! {
            #[event(name = "event", revision = 0)]
            struct Event;
        };

        let err = derive(input).unwrap_err();

        assert_eq!(err.to_string(), "number would be zero for non-zero type");
    }

    #[test]
    fn errors_on_u16_overflowed_revision() {
        let input = parse_quote! {
            #[event(name = "event", revision = 4294967295)]
            struct Event;
        };

        let err = derive(input).unwrap_err();

        assert_eq!(err.to_string(), "number too large to fit in target type");
    }

    #[test]
    fn errors_on_enum() {
        let input = parse_quote! {
            #[event(name = "event", revision = 1)]
            enum Event {
                Event1(Event1),
            }
        };

        let err = derive(input).unwrap_err();

        assert_eq!(err.to_string(), "only structs are allowed");
    }
}
