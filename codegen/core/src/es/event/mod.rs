//! `#[derive(Event)]` macro implementation.

pub mod versioned;

use std::convert::TryFrom;

use proc_macro2::TokenStream;
use quote::quote;
use syn::spanned::Spanned;
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
    /// Indicator whether to ignore this enum variant for code generation.
    #[parse(ident, alias = skip)]
    pub ignore: Option<syn::Ident>,
}

/// Representation of an enum implementing [`Event`], used for code generation.
///
/// [`Event`]: arcana_core::es::event::Event
#[derive(Debug, ToTokens)]
#[to_tokens(append(impl_event, gen_uniqueness_glue_code))]
pub struct Definition {
    /// [`syn::Ident`](struct@syn::Ident) of this enum's type.
    pub ident: syn::Ident,

    /// [`syn::Generics`] of this Enum's type.
    pub generics: syn::Generics,

    /// Single-[`Field`] [`Variant`]s of this enum to consider in code
    /// generation.
    ///
    /// [`Field`]: syn::Field
    /// [`Variant`]: syn::Variant
    pub variants: Vec<syn::Variant>,

    /// Indicator whether this enum has variants marked with `#[event(ignore)]`
    /// attribute.
    pub has_ignored_variants: bool,
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
    /// Parses and validates [`syn::Variant`] its [`VariantAttrs`].
    ///
    /// # Errors
    ///
    /// - If [`VariantAttrs`] failed to parse.
    /// - If [`syn::Variant`] doesn't have exactly one unnamed 1 [`syn::Field`]
    ///   and is not ignored.
    fn parse_variant(
        variant: &syn::Variant,
    ) -> syn::Result<Option<syn::Variant>> {
        let attrs = VariantAttrs::parse_attrs("event", variant)?;
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

        Ok(Some(variant.clone()))
    }

    /// Replaces [`syn::Type`] generics with default values.
    ///
    /// - [`syn::Lifetime`] -> `'static`;
    /// - [`syn::Type`] -> `()`;
    /// - [`syn::Binding`] -> `Ident = ()`;
    /// - `Fn(A, B) -> C` -> `Fn((), ()) -> ()`.
    fn replace_type_generics_with_default_values(ty: &syn::Type) -> syn::Type {
        match ty {
            syn::Type::Path(path) => {
                let mut path = path.clone();

                for segment in &mut path.path.segments {
                    match &mut segment.arguments {
                        syn::PathArguments::AngleBracketed(generics) => {
                            for arg in &mut generics.args {
                                match arg {
                                    syn::GenericArgument::Lifetime(l) => {
                                        *l = syn::parse_quote!('static);
                                    }
                                    syn::GenericArgument::Type(ty) => {
                                        *ty = syn::parse_quote!(());
                                    }
                                    syn::GenericArgument::Binding(bind) => {
                                        bind.ty = syn::parse_quote!(());
                                    }
                                    syn::GenericArgument::Const(_)
                                    | syn::GenericArgument::Constraint(_) => {}
                                }
                            }
                        }
                        syn::PathArguments::Parenthesized(paren) => {
                            paren.output = syn::parse_quote!(());
                            for input in &mut paren.inputs {
                                *input = syn::parse_quote!(());
                            }
                        }
                        syn::PathArguments::None => {}
                    }
                }

                syn::Type::Path(path)
            }
            ty => ty.clone(),
        }
    }

    /// Replaces [`syn::Generics`] with default values.
    ///
    /// - [`syn::Lifetime`] -> `'static`;
    /// - [`syn::Type`] -> `()`.
    fn replace_generics_with_default_values(
        generics: &syn::Generics,
    ) -> TokenStream {
        let generics = generics.params.iter().map(|param| match param {
            syn::GenericParam::Lifetime(_) => quote! { 'static },
            syn::GenericParam::Type(_) => quote! { () },
            syn::GenericParam::Const(c) => quote! { #c },
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

        let var = self.variants.iter().map(|v| &v.ident).collect::<Vec<_>>();

        let unreachable_arm = self.has_ignored_variants.then(|| {
            quote! {
                _ => unreachable!(),
            }
        });

        quote! {
            #[automatically_derived]
            impl #impl_gens ::arcana::es::Event for #ty#ty_gens #where_clause {
                fn name(&self) -> ::arcana::es::event::Name {
                    match self {
                        #( Self::#var(f) => ::arcana::es::Event::name(f), )*
                        #unreachable_arm
                    }
                }

                fn version(&self) -> ::arcana::es::event::Version {
                    match self {
                        #( Self::#var(f) => ::arcana::es::Event::version(f), )*
                        #unreachable_arm
                    }
                }
            }
        }
    }

    /// Generates functions, that returns array composed from arrays of all enum
    /// variants.
    ///
    /// Checks uniqueness of all [`Event::name`][0]s and [`Event::version`][1]s.
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

        let default_generics =
            Self::replace_generics_with_default_values(&self.generics);

        let (var_ty, var_ty_with_default_generics): (Vec<_>, Vec<_>) = self
            .variants
            .iter()
            .flat_map(|v| &v.fields)
            .map(|f| {
                (
                    &f.ty,
                    Self::replace_type_generics_with_default_values(&f.ty),
                )
            })
            .unzip();

        // TODO: use `Self::__arcana_events()` inside impl, once
        //       https://github.com/rust-lang/rust/issues/57775 is resolved.
        quote! {
            #[automatically_derived]
            #[doc(hidden)]
            impl #impl_gens ::arcana::codegen::UniqueEvents for #ty#ty_gens
                 #where_clause
            {
                #[doc(hidden)]
                const COUNT: usize =
                    #( <#var_ty as ::arcana::codegen::UniqueEvents>::COUNT )+*;
            }

            #[automatically_derived]
            #[doc(hidden)]
            impl #ty #default_generics {
                #[doc(hidden)]
                pub const fn __arcana_events() -> [
                    (&'static str, u16);
                    <Self as ::arcana::codegen::UniqueEvents>::COUNT
                ] {
                    let mut res = [
                        ("", 0);
                        <Self as ::arcana::codegen::UniqueEvents>::COUNT
                    ];

                    let mut i = 0;
                    #({
                        let events =
                            <#var_ty_with_default_generics>::__arcana_events();
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

            ::arcana::codegen::sa::const_assert!(
                !::arcana::codegen::unique_events::has_duplicates(
                    #ty::#default_generics::__arcana_events()
                )
            );
        }
    }
}

#[cfg(test)]
mod spec {
    use super::{derive, quote};

    #[test]
    fn derives_enum_impl() {
        let input = syn::parse_quote! {
            enum Event {
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
            #[doc(hidden)]
            impl ::arcana::codegen::UniqueEvents for Event {
                #[doc(hidden)]
                const COUNT: usize =
                    <FileEvent as ::arcana::codegen::UniqueEvents>::COUNT +
                    <ChatEvent as ::arcana::codegen::UniqueEvents>::COUNT;
            }

            #[automatically_derived]
            #[doc(hidden)]
            impl Event<> {
                #[doc(hidden)]
                pub const fn __arcana_events() -> [
                    (&'static str, u16);
                    <Self as ::arcana::codegen::UniqueEvents>::COUNT
                ] {
                    let mut res = [
                        ("", 0);
                        <Self as ::arcana::codegen::UniqueEvents>::COUNT
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

            ::arcana::codegen::sa::const_assert!(
                !::arcana::codegen::unique_events::has_duplicates(
                    Event::<>::__arcana_events()
                )
            );
        };

        assert_eq!(derive(input).unwrap().to_string(), output.to_string());
    }

    #[test]
    fn derives_enum_with_generics_impl() {
        let input = syn::parse_quote! {
            enum Event<'a, F, C> {
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
            #[doc(hidden)]
            impl<'a, F, C> ::arcana::codegen::UniqueEvents for Event<'a, F, C> {
                #[doc(hidden)]
                const COUNT: usize =
                    <FileEvent<'a, F> as ::arcana::codegen::UniqueEvents>
                        ::COUNT +
                    <ChatEvent<'a, C> as ::arcana::codegen::UniqueEvents>
                        ::COUNT;
            }

            #[automatically_derived]
            #[doc(hidden)]
            impl Event<'static, (), ()> {
                #[doc(hidden)]
                pub const fn __arcana_events() -> [
                    (&'static str, u16);
                    <Self as ::arcana::codegen::UniqueEvents>::COUNT
                ] {
                    let mut res = [
                        ("", 0);
                        <Self as ::arcana::codegen::UniqueEvents>::COUNT
                    ];

                    let mut i = 0;

                    {
                        let events =
                            < FileEvent<'static, ()> >::__arcana_events();
                        let mut j = 0;
                        while j < events.len() {
                            res[i] = events[j];
                            j += 1;
                            i += 1;
                        }
                    }

                    {
                        let events =
                            < ChatEvent<'static, ()> >::__arcana_events();
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

            ::arcana::codegen::sa::const_assert!(
                !::arcana::codegen::unique_events::has_duplicates(
                    Event::<'static, (), ()>::__arcana_events()
                )
            );
        };

        assert_eq!(derive(input).unwrap().to_string(), output.to_string());
    }

    #[test]
    fn skip_unique_check_on_variant() {
        let input_skip = syn::parse_quote! {
            enum Event {
                File(FileEvent),
                Chat(ChatEvent),
                #[event(skip)]
                _NonExhaustive
            }
        };

        let input_ignore = syn::parse_quote! {
            enum Event {
                File(FileEvent),
                Chat(ChatEvent),
                #[event(ignore)]
                _NonExhaustive
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
            #[doc(hidden)]
            impl ::arcana::codegen::UniqueEvents for Event {
                #[doc(hidden)]
                const COUNT: usize =
                    <FileEvent as ::arcana::codegen::UniqueEvents>::COUNT +
                    <ChatEvent as ::arcana::codegen::UniqueEvents>::COUNT;
            }

            #[automatically_derived]
            #[doc(hidden)]
            impl Event<> {
                #[doc(hidden)]
                pub const fn __arcana_events() -> [
                    (&'static str, u16);
                    <Self as ::arcana::codegen::UniqueEvents>::COUNT
                ] {
                    let mut res = [
                        ("", 0);
                        <Self as ::arcana::codegen::UniqueEvents>::COUNT
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

            ::arcana::codegen::sa::const_assert!(
                !::arcana::codegen::unique_events::has_duplicates(
                    Event::<>::__arcana_events()
                )
            );
        };

        let input_skip = derive(input_skip).unwrap().to_string();
        let input_ignore = derive(input_ignore).unwrap().to_string();
        assert_eq!(input_skip, input_ignore);
        assert_eq!(input_skip, output.to_string());
    }

    #[test]
    fn errors_on_multiple_fields_in_variant() {
        let input = syn::parse_quote! {
            enum Event {
                Event1(Event1),
                Event2 {
                    event: Event2,
                    second_field: Event3,
                }
            }
        };

        let error = derive(input).unwrap_err();

        assert_eq!(
            format!("{}", error),
            "enum variants must have exactly 1 field",
        );
    }

    #[test]
    fn errors_on_struct() {
        let input = syn::parse_quote! {
            struct Event;
        };

        let error = derive(input).unwrap_err();

        assert_eq!(
            format!("{}", error),
            "expected enum only, \
             consider using `arcana::es::event::Versioned` for structs",
        );
    }
}
