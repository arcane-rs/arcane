//! `#[derive(Event)]` macro implementation for enums.

use std::iter;

use itertools::Itertools as _;
use proc_macro2::TokenStream;
use quote::quote;
use syn::{parse_quote, spanned::Spanned as _};
use synthez::{ParseAttrs, ToTokens};

#[cfg(all(doc, feature = "doc"))]
use arcane_core::es::{event, Event};

/// Attributes of the `#[derive(Event)]` macro placed on an enum.
#[derive(Debug, Default, ParseAttrs)]
pub struct Attrs {
    /// Indicator whether an enum should be treated as an [`event::Revisable`].
    #[parse(ident, alias = rev)]
    pub revision: Option<syn::Ident>,
}

/// Representation of an enum implementing [`Event`] (and [`event::Revisable`],
/// optionally), used for the code generation.
#[derive(Debug, ToTokens)]
#[to_tokens(append(
    impl_event,
    impl_event_revisable,
    impl_event_sourced,
    impl_into_raw,
    impl_from_raw,
    gen_uniqueness_assertion
))]
#[cfg_attr(
    feature = "reflect",
    to_tokens(append(impl_reflect_static, impl_reflect_concrete))
)]
pub struct Definition {
    /// [`syn::Ident`](struct@syn::Ident) of this enum's type.
    pub ident: syn::Ident,

    /// [`syn::Generics`] of this enum's type.
    pub generics: syn::Generics,

    /// [`Variant`]s of this enum.
    pub variants: Vec<Variant>,

    /// Indicator whether this enum has any [`Variant`]s marked with
    /// `#[event(ignore)]` attribute.
    pub has_ignored_variants: bool,

    /// Indicator whether this enum should implement [`event::Revisable`].
    pub is_revisable: bool,
}

impl TryFrom<syn::DeriveInput> for Definition {
    type Error = syn::Error;

    fn try_from(input: syn::DeriveInput) -> syn::Result<Self> {
        let attrs = Attrs::parse_attrs("event", &input)?;

        let syn::Data::Enum(data) = &input.data else {
            return Err(syn::Error::new(
                input.span(),
                "only enums are allowed",
            ));
        };

        let variants = data
            .variants
            .iter()
            .filter_map(|v| Variant::parse(v).transpose())
            .collect::<syn::Result<Vec<_>>>()?;
        if variants.is_empty() {
            return Err(syn::Error::new(
                input.span(),
                "enum must have at least one non-ignored variant",
            ));
        }

        let has_ignored_variants = variants.len() < data.variants.len();

        Ok(Self {
            ident: input.ident,
            generics: input.generics,
            variants,
            has_ignored_variants,
            is_revisable: attrs.revision.is_some(),
        })
    }
}

impl Definition {
    /// Substitutes the provided [`syn::Generics`] with trivial types.
    ///
    /// - [`syn::Lifetime`] -> `'static`;
    /// - [`syn::Type`] -> `()`.
    ///
    /// [`syn::Lifetime`]: struct@syn::Lifetime
    fn substitute_generics_trivially(generics: &syn::Generics) -> TokenStream {
        use syn::GenericParam::{Const, Lifetime, Type};

        let generics = generics.params.iter().map(|p| match p {
            Lifetime(_) => quote! { 'static },
            Type(_) => quote! { () },
            Const(c) => quote! { #c },
        });

        quote! { < #( #generics ),* > }
    }

    /// Shadows the provided [`syn::Generics`] with `type T = ();` aliases.
    /// This required for `const` contexts, where generic type parameters cannot
    /// be passed correctly.
    // TODO: Remove this, once rust-lang/rust#57775 is resolved:
    //       https://github.com/rust-lang/rust/issues/57775
    fn shadow_generics_trivially(generics: &syn::Generics) -> TokenStream {
        let shadow_ty = generics.type_params().map(|p| {
            let ident = &p.ident;

            quote! { type #ident = (); }
        });

        quote! { #( #shadow_ty )* }
    }

    /// Generates code of an [`Event`] trait implementation, by simply matching
    /// over each enum variant, which is expected to be itself an [`Event`]
    /// implementer.
    ///
    /// [`Event`]: event::Event
    #[must_use]
    pub fn impl_event(&self) -> TokenStream {
        let ty = &self.ident;
        let (impl_gens, ty_gens, where_clause) = self.generics.split_for_impl();

        let var_ident = self.variants.iter().map(|v| &v.ident);

        let unreachable_arm = self.has_ignored_variants.then(|| {
            quote! { _ => unreachable!(), }
        });

        quote! {
            #[automatically_derived]
            impl #impl_gens ::arcane::es::Event for #ty #ty_gens #where_clause {
                fn name(&self) -> ::arcane::es::event::Name {
                    match self {
                        #(
                            Self::#var_ident(f) => ::arcane::es::Event::name(f),
                        )*
                        #unreachable_arm
                    }
                }
            }
        }
    }

    /// Generates code of an [`event::Revisable`] trait implementation, by
    /// simply matching over each enum variant, which is expected to be itself
    /// an [`event::Revisable`] implementer, and using the
    /// [`event::Revisable::Revision`] type of the first variant.
    #[must_use]
    pub fn impl_event_revisable(&self) -> TokenStream {
        if !self.is_revisable {
            return TokenStream::new();
        }

        let ident = &self.ident;
        let (impl_gens, ty_gens, where_clause) = self.generics.split_for_impl();

        let first_var_ty = self.variants.iter().map(|v| &v.ty).next();

        let where_clause = {
            let mut clause = where_clause
                .cloned()
                .unwrap_or_else(|| parse_quote! { where });
            for v in &self.variants {
                let var_ty = &v.ty;

                clause.predicates.push(parse_quote! {
                    #var_ty: ::arcane::es::event::Revisable
                });
                clause.predicates.push(parse_quote! {
                    ::arcane::es::event::RevisionOf<#first_var_ty>:
                        From<::arcane::es::event::RevisionOf<#var_ty>>
                });
            }
            clause
        };

        let var_ident = self.variants.iter().map(|v| &v.ident);

        let unreachable_arm = self.has_ignored_variants.then(|| {
            quote! { _ => unreachable!(), }
        });

        quote! {
            #[automatically_derived]
            impl #impl_gens ::arcane::es::event::Revisable for #ident #ty_gens
                 #where_clause
            {
                type Revision = ::arcane::es::event::RevisionOf<#first_var_ty>;

                fn revision(&self) -> Self::Revision {
                    match self {
                        #(
                            Self::#var_ident(f) => Self::Revision::from(
                                ::arcane::es::event::Revisable::revision(f)
                            ),
                        )*
                        #unreachable_arm
                    }
                }
            }
        }
    }

    /// Generates code of an [`event::Sourced`] trait blanket implementation, by
    /// simply matching each enum variant, which is expected to have itself an
    /// an [`event::Sourced`] implementation.
    #[must_use]
    pub fn impl_event_sourced(&self) -> TokenStream {
        let ty = &self.ident;
        let (_, ty_gens, _) = self.generics.split_for_impl();
        let turbofish_gens = ty_gens.as_turbofish();

        let var_tys = self.variants.iter().map(|v| {
            let var_ty = &v.ty;
            match v.sourcing {
                VariantEventSourcing::Initialized => quote! {
                    ::arcane::es::event::Initial<#var_ty>
                },
                VariantEventSourcing::Sourced => quote! { #var_ty },
            }
        });

        let mut ext_gens = self.generics.clone();
        ext_gens.params.push(parse_quote! { __S });
        ext_gens.make_where_clause().predicates.push(parse_quote! {
            Self: #( ::arcane::es::event::Sourced<#var_tys> )+*
        });
        let (impl_gens, _, where_clause) = ext_gens.split_for_impl();

        let arms = self.variants.iter().map(|v| {
            let var = &v.ident;
            let var_ty = &v.ty;

            let event = match v.sourcing {
                VariantEventSourcing::Initialized => quote! {
                    <::arcane::es::event::Initial<#var_ty>
                     as ::arcane::RefCast>::ref_cast(f)
                },
                VariantEventSourcing::Sourced => quote! { f },
            };
            quote! {
                #ty #turbofish_gens::#var(f) => {
                    ::arcane::es::event::Sourced::apply(self, #event);
                },
            }
        });
        let unreachable_arm = self.has_ignored_variants.then(|| {
            quote! { _ => unreachable!(), }
        });

        quote! {
            #[automatically_derived]
            impl #impl_gens ::arcane::es::event::Sourced<#ty #ty_gens>
             for Option<__S> #where_clause
            {
                fn apply(&mut self, event: &#ty #ty_gens) {
                    match event {
                        #( #arms )*
                        #unreachable_arm
                    }
                }
            }
        }
    }

    #[cfg(feature = "reflect")]
    /// Generates code of an [`event::reflect::Static`] trait implementation.
    #[must_use]
    pub fn impl_reflect_static(&self) -> TokenStream {
        let ty = &self.ident;
        let (impl_gens, ty_gens, where_clause) = self.generics.split_for_impl();

        let var_ty = self.variants.iter().map(|f| &f.ty);

        let subst_gen_types = Self::shadow_generics_trivially(&self.generics);

        quote! {
            #[automatically_derived]
            impl #impl_gens ::arcane::es::event::reflect::Static
             for #ty #ty_gens #where_clause
            {
                const NAMES: &'static [::arcane::es::event::Name] = {
                    #subst_gen_types
                    ::arcane::es::event::codegen::const_concat_slices!(
                        #(
                            <#var_ty as ::arcane::es::event::reflect::Static>
                                ::NAMES,
                        )*
                    )
                };
            }
        }
    }

    #[cfg(feature = "reflect")]
    /// Generates code of an [`event::reflect::Concrete`] trait implementation.
    #[must_use]
    pub fn impl_reflect_concrete(&self) -> TokenStream {
        if !self.is_revisable {
            return TokenStream::new();
        }

        let ty = &self.ident;
        let (impl_gens, ty_gens, where_clause) = self.generics.split_for_impl();

        let var_ty = self.variants.iter().map(|f| &f.ty);

        let subst_gen_types = Self::shadow_generics_trivially(&self.generics);

        quote! {
            #[automatically_derived]
            impl #impl_gens ::arcane::es::event::reflect::Concrete
             for #ty #ty_gens #where_clause
            {
                // TODO: Replace with `::arcane::es::event::RevisionOf<Self>`
                //       once rust-lang/rust#57775 is resolved:
                //       https://github.com/rust-lang/rust/issues/57775
                const REVISIONS: &'static [::arcane::es::event::Version] = {
                    #subst_gen_types
                    ::arcane::es::event::codegen::const_concat_slices!(
                        #(
                            <#var_ty as ::arcane::es::event::reflect::Concrete>
                                ::REVISIONS,
                        )*
                    )
                };
            }
        }
    }

    /// Generates code allows to convert this [`Event`] into its [`Raw`]
    /// representation.
    ///
    /// [`Raw`]: event::Raw
    #[must_use]
    pub fn impl_into_raw(&self) -> TokenStream {
        let ty = &self.ident;
        let first_var_ty = self.variants.first().map(|v| &v.ty);

        let (_, ty_gens, _) = self.generics.split_for_impl();
        let generics = {
            let mut generics = self.generics.clone();
            generics.params.push(parse_quote! { '__raw });
            generics.params.push(parse_quote! { __Data });

            // `dedup_by` is required here to avoid
            // `A: TryFrom<B, Error = <A as TryFrom<B>>::Error>`
            // recursion. It is only occurs if the same expression is in
            // recursion and doesn't affect type aliases.
            let try_from_variants = self
                .variants
                .iter()
                .dedup_by(|a, b| a.ty == b.ty)
                .skip(1)
                .map(|v| {
                    let var_ty = &v.ty;

                    parse_quote! {
                        __Data: ::std::convert::TryFrom<#var_ty, Error = <
                            __Data as ::std::convert::TryFrom<#first_var_ty>
                        >::Error>
                    }
                });

            let where_clause = generics
                .where_clause
                .get_or_insert_with(|| parse_quote! { where });
            where_clause.predicates.extend(
                iter::once::<syn::WherePredicate>(parse_quote! {
                    __Data: ::std::convert::TryFrom<#first_var_ty>
                })
                .chain(try_from_variants),
            );

            generics
        };
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        let (revision_ty, revision) = self
            .is_revisable
            .then(|| {
                let revision_ty = quote! {
                    ::arcane::es::event::RevisionOf<#ty #ty_gens>
                };
                let revision = quote! {
                    <#ty #ty_gens
                     as ::arcane::es::event::Revisable>::revision(&event)
                };

                (revision_ty, revision)
            })
            .unwrap_or_else(|| (quote! { () }, quote! { () }));

        let into_variant_arms = self.variants.iter().map(|v| {
            let var_ident = &v.ident;
            let var_ty = &v.ty;

            quote! {
                #ty::#var_ident(ev) => <
                    __Data as ::std::convert::TryFrom<#var_ty>
                >::try_from(ev)?,
            }
        });
        let unreachable_arm = self.has_ignored_variants.then(|| {
            quote! { _ => unreachable!(), }
        });

        quote! {
            #[automatically_derived]
            impl #impl_gens ::std::convert::TryFrom<#ty #ty_gens>
             for ::arcane::es::event::Raw<'__raw, __Data, #revision_ty>
                 #where_clause
            {
                type Error = <
                    __Data as ::std::convert::TryFrom<#first_var_ty>
                >::Error;

                fn try_from(event: #ty #ty_gens)
                    -> ::std::result::Result<Self, Self::Error>
                {
                    Ok(Self {
                        name: ::std::borrow::Cow::from(<
                            #ty #ty_gens as ::arcane::es::Event
                        >::name(&event)),
                        revision: #revision,
                        data: match event {
                            #( #into_variant_arms )*
                            #unreachable_arm
                        },
                    })
                }
            }
        }
    }

    /// Generates code allows to construct this [`Event`] from its [`Raw`]
    /// representation.
    ///
    /// [`Raw`]: event::Raw
    #[must_use]
    pub fn impl_from_raw(&self) -> TokenStream {
        let ty = &self.ident;
        let first_var_ty = self.variants.first().map(|v| &v.ty);

        let (_, ty_gens, _) = self.generics.split_for_impl();

        let (revision_ty, check_revision) = self
            .is_revisable
            .then(|| {
                let revision_ty =
                    quote! { ::arcane::es::event::RevisionOf<#ty #ty_gens> };
                let check_revision = quote! {
                    && *var_rev == <
                        #revision_ty as ::std::string::ToString
                    >::to_string(&raw.revision)
                };

                (revision_ty, check_revision)
            })
            .unwrap_or_else(|| (quote! { () }, quote! {}));

        let generics = {
            let mut generics = self.generics.clone();
            generics.params.push(parse_quote! { '__raw });
            generics.params.push(parse_quote! { __Data });

            // `dedup_by` is required here to avoid
            // `A: TryFrom<B, Error = <A as TryFrom<B>>::Error>`
            // recursion. It is only occurs if the same expression is in
            // recursion and doesn't affect type aliases.
            let variants_try_from = self
                .variants
                .iter()
                .dedup_by(|a, b| a.ty == b.ty)
                .skip(1)
                .map(|v| {
                    let var_ty = &v.ty;

                    parse_quote! {
                        #var_ty: ::std::convert::TryFrom<__Data, Error = <
                            #first_var_ty as ::std::convert::TryFrom<__Data>
                        >::Error>
                    }
                });

            let where_clause = generics
                .where_clause
                .get_or_insert_with(|| parse_quote! { where });
            where_clause.predicates.extend(
                iter::once::<syn::WherePredicate>(parse_quote! {
                    #first_var_ty: ::std::convert::TryFrom<__Data>
                })
                .chain(variants_try_from),
            );

            generics
        };
        let (impl_gens, _, where_clause) = generics.split_for_impl();

        // `dedup_by` is required to improve runtime performance by avoiding
        // double-checks of the variants guaranteed to be duplicates.
        let try_from_variant =
            self.variants.iter().dedup_by(|a, b| a.ty == b.ty).map(|v| {
                let var_ident = &v.ident;
                let var_ty = &v.ty;

                quote! {
                    for (_, var_name, var_rev) in
                        <#var_ty as ::arcane::es::event::codegen::Reflect>::META
                    {
                        if *var_name == raw.name #check_revision {
                            return <
                                #var_ty as ::std::convert::TryFrom<__Data>
                            >::try_from(raw.data)
                            .map(Self:: #var_ident)
                            .map_err(
                                ::arcane::es::event::FromRawError::FromDataError
                            );
                        }
                    }
                }
            });

        quote! {
            #[automatically_derived]
            impl #impl_gens ::std::convert::TryFrom<
               ::arcane::es::event::Raw<'__raw, __Data, #revision_ty>
            > for #ty #ty_gens
            #where_clause
            {
                type Error = ::arcane::es::event::FromRawError<
                    <#first_var_ty as TryFrom<__Data>>::Error,
                    #revision_ty
                >;

                fn try_from(
                    raw: ::arcane::es::event::Raw<'__raw, __Data, #revision_ty>
                ) -> ::std::result::Result<Self, Self::Error>
                {
                    #( #try_from_variant )*

                    Err(::arcane::es::event::FromRawError::UnknownEvent {
                        name: raw.name.to_string(),
                        revision: raw.revision,
                    })
                }
            }
        }
    }

    /// Generates non-public machinery code used to statically check whether all
    /// the [`Event::name`]s and [`event::Revisable::revision`]s pairs
    /// correspond to a single Rust type.
    #[must_use]
    pub fn gen_uniqueness_assertion(&self) -> TokenStream {
        let ty = &self.ident;
        let (impl_gens, ty_gens, where_clause) = self.generics.split_for_impl();

        let var_ty = self.variants.iter().map(|f| &f.ty);

        // TODO: Use `has_different_types_with_same_name_and_ver` inside impl
        //       instead of type params substitution, once rust-lang/rust#57775
        //       is resolved: https://github.com/rust-lang/rust/issues/57775
        let ty_subst_gens = Self::substitute_generics_trivially(&self.generics);
        let subst_gen_types = Self::shadow_generics_trivially(&self.generics);

        let codegen = quote! { ::arcane::es::event::codegen };
        quote! {
            #[automatically_derived]
            #[doc(hidden)]
            impl #impl_gens #codegen ::Reflect for #ty #ty_gens #where_clause {
                #[doc(hidden)]
                const META: &'static [
                    (&'static str, &'static str, &'static str)
                ] = {
                    #subst_gen_types
                    #codegen ::const_concat_slices!(
                        #( <#var_ty as #codegen ::Reflect>::META, )*
                    )
                };
            }

            #[automatically_derived]
            #[doc(hidden)]
            const _: () = ::std::assert!(
                !#codegen ::has_different_types_with_same_name_and_revision
                          ::<#ty #ty_subst_gens>(),
                "having different `Event` types with the same name \
                 and revision inside a single enum is forbidden",
            );
        }
    }
}

/// Attributes of `#[derive(Event)]` macro placed on a [`Variant`].
#[derive(Debug, Default, ParseAttrs)]
pub struct VariantAttrs {
    /// Indicator whether this enum variant should be used as
    /// [`event::Initialized`] rather than [`event::Sourced`].
    #[parse(ident, alias = initial)]
    pub init: Option<syn::Ident>,

    /// Indicator whether to ignore this enum variant for code generation.
    #[parse(ident, alias = skip)]
    pub ignore: Option<syn::Ident>,
}

/// Type of event sourcing the [`Variant`] is using.
#[derive(Clone, Copy, Debug)]
pub enum VariantEventSourcing {
    /// [`Variant`] used as [`event::Initialized`].
    Initialized,

    /// [`Variant`] used as [`event::Sourced`].
    Sourced,
}

/// Representation of a single-fielded variant of an enum deriving
/// `#[derive(Event)]`, used for the code generation.
#[derive(Debug)]
pub struct Variant {
    /// [`syn::Ident`](struct@syn::Ident) of this [`Variant`].
    pub ident: syn::Ident,

    /// [`syn::Type`] of this [`Variant`].
    pub ty: syn::Type,

    /// [`VariantEventSourcing`] type of this [`Variant`].
    pub sourcing: VariantEventSourcing,
}

impl Variant {
    /// Validates the given [`syn::Variant`], parses its [`VariantAttrs`], and
    /// returns a [`Variant`] if the validation succeeds.
    ///
    /// # Errors
    ///
    /// - If [`VariantAttrs`] failed to parse.
    /// - If [`VariantAttrs::init`] and [`VariantAttrs::ignore`] were specified
    ///   simultaneously.
    /// - If [`syn::Variant`] doesn't have exactly one unnamed 1 [`syn::Field`]
    ///   and is not ignored.
    pub fn parse(variant: &syn::Variant) -> syn::Result<Option<Self>> {
        let attrs = VariantAttrs::parse_attrs("event", variant)?;

        if let Some(init) = &attrs.init {
            if attrs.ignore.is_some() {
                return Err(syn::Error::new(
                    init.span(),
                    "`init` and `ignore`/`skip` arguments are mutually \
                     exclusive",
                ));
            }
        }

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

        let field = variant.fields.iter().next().ok_or_else(|| {
            syn::Error::new(
                variant.span(),
                "enum variants must have exactly 1 field",
            )
        })?;
        let sourcing = attrs.init.map_or(VariantEventSourcing::Sourced, |_| {
            VariantEventSourcing::Initialized
        });

        Ok(Some(Self {
            ident: variant.ident.clone(),
            ty: field.ty.clone(),
            sourcing,
        }))
    }
}

#[cfg(test)]
mod spec {
    use proc_macro2::TokenStream;
    use quote::{quote, ToTokens};
    use syn::parse_quote;

    use super::Definition;

    /// Expands the `#[derive(Event)]` macro on the provided enum and returns
    /// the generated code.
    fn derive(input: TokenStream) -> syn::Result<TokenStream> {
        let input = syn::parse2::<syn::DeriveInput>(input)?;
        Ok(Definition::try_from(input)?.into_token_stream())
    }

    #[expect(clippy::too_many_lines, reason = "OK for macro expansion test")]
    #[test]
    fn derives_enum_impl() {
        let input = parse_quote! {
            enum Event {
                #[event(init)]
                File(FileEvent),
                Chat(ChatEvent),
            }
        };

        let mut output = quote! {
            #[automatically_derived]
            impl ::arcane::es::Event for Event {
                fn name(&self) -> ::arcane::es::event::Name {
                    match self {
                        Self::File(f) => ::arcane::es::Event::name(f),
                        Self::Chat(f) => ::arcane::es::Event::name(f),
                    }
                }
            }

            #[automatically_derived]
            impl<__S> ::arcane::es::event::Sourced<Event> for Option<__S>
            where
                Self: ::arcane::es::event::Sourced<
                          ::arcane::es::event::Initial<FileEvent>
                      > +
                      ::arcane::es::event::Sourced<ChatEvent>
            {
                fn apply(&mut self, event: &Event) {
                    match event {
                        Event::File(f) => {
                            ::arcane::es::event::Sourced::apply(
                                self,
                                <::arcane::es::event::Initial<FileEvent>
                                 as ::arcane::RefCast>::ref_cast(f)
                            );
                        },
                        Event::Chat(f) => {
                            ::arcane::es::event::Sourced::apply(self, f);
                        },
                    }
                }
            }

            #[automatically_derived]
            impl<'__raw, __Data> ::std::convert::TryFrom<Event>
                             for ::arcane::es::event::Raw<'__raw, __Data, ()>
            where
                __Data: ::std::convert::TryFrom<FileEvent>,
                __Data: ::std::convert::TryFrom<
                    ChatEvent,
                    Error = <
                        __Data as ::std::convert::TryFrom<FileEvent>
                    >::Error
                >
            {
                type Error = <
                    __Data as ::std::convert::TryFrom<FileEvent>
                >::Error;

                fn try_from(event: Event)
                    -> ::std::result::Result<Self, Self::Error>
                {
                    Ok(Self {
                        name: ::std::borrow::Cow::from(<
                            Event as ::arcane::es::Event
                        >::name(&event)),
                        revision: (),
                        data: match event {
                            Event::File(ev) => <
                                __Data as ::std::convert::TryFrom<FileEvent>
                            >::try_from(ev)?,
                            Event::Chat(ev) => <
                                __Data as ::std::convert::TryFrom<ChatEvent>
                            >::try_from(ev)?,
                        },
                    })
                }
            }

            #[automatically_derived]
            impl<'__raw, __Data> ::std::convert::TryFrom<
                ::arcane::es::event::Raw<'__raw, __Data, ()>
            > for Event
            where
                FileEvent: ::std::convert::TryFrom<__Data>,
                ChatEvent: ::std::convert::TryFrom<
                    __Data,
                    Error = <
                        FileEvent as ::std::convert::TryFrom<__Data>
                    >::Error
                >
            {
                type Error = ::arcane::es::event::FromRawError<
                    <FileEvent as TryFrom<__Data>>::Error,
                    ()
                >;

                fn try_from(
                    raw: ::arcane::es::event::Raw<'__raw, __Data, ()>
                ) -> ::std::result::Result<Self, Self::Error> {
                    for (_, var_name, var_rev) in <
                        FileEvent as ::arcane::es::event::codegen::Reflect
                    >::META {
                        if *var_name == raw.name {
                            return <
                                FileEvent as ::std::convert::TryFrom<__Data>
                            >::try_from(raw.data)
                            .map(Self::File)
                            .map_err(
                                ::arcane::es::event::FromRawError::FromDataError
                            );
                        }
                    }

                    for (_, var_name, var_rev) in <
                        ChatEvent as ::arcane::es::event::codegen::Reflect
                    >::META {
                        if *var_name == raw.name {
                            return <
                                ChatEvent as ::std::convert::TryFrom<__Data>
                            >::try_from(raw.data)
                            .map(Self::Chat)
                            .map_err(
                                ::arcane::es::event::FromRawError::FromDataError
                            );
                        }
                    }

                    Err(::arcane::es::event::FromRawError::UnknownEvent {
                        name: raw.name.to_string(),
                        revision: raw.revision,
                    })
                }
            }

            #[automatically_derived]
            #[doc(hidden)]
            impl ::arcane::es::event::codegen::Reflect for Event {
                #[doc(hidden)]
                const META: &'static [
                    (&'static str, &'static str, &'static str)
                ] = {
                    ::arcane::es::event::codegen::const_concat_slices!(
                        <FileEvent
                         as ::arcane::es::event::codegen::Reflect>::META,
                        <ChatEvent
                         as ::arcane::es::event::codegen::Reflect>::META,
                    )
                };
            }

            #[automatically_derived]
            #[doc(hidden)]
            const _: () = ::std::assert!(
                !::arcane::es::event::codegen
                 ::has_different_types_with_same_name_and_revision
                 ::<Event<> >(),
                "having different `Event` types with the same name \
                 and revision inside a single enum is forbidden",
            );
        };
        if cfg!(feature = "reflect") {
            output.extend([quote! {
                #[automatically_derived]
                impl ::arcane::es::event::reflect::Static for Event {
                    const NAMES: &'static [::arcane::es::event::Name] = {
                        ::arcane::es::event::codegen::const_concat_slices!(
                            <FileEvent
                             as ::arcane::es::event::reflect::Static>::NAMES,
                            <ChatEvent
                             as ::arcane::es::event::reflect::Static>::NAMES,
                        )
                    };
                }
            }]);
        }

        assert_eq!(derive(input).unwrap().to_string(), output.to_string());
    }

    #[expect(clippy::too_many_lines, reason = "OK for macro expansion test")]
    #[test]
    fn derives_enum_impl_with_duplicate_variants() {
        let input = parse_quote! {
            enum Event {
                #[event(init)]
                File(FileEvent),
                DupFile(FileEvent),
            }
        };

        let mut output = quote! {
            #[automatically_derived]
            impl ::arcane::es::Event for Event {
                fn name(&self) -> ::arcane::es::event::Name {
                    match self {
                        Self::File(f) => ::arcane::es::Event::name(f),
                        Self::DupFile(f) => ::arcane::es::Event::name(f),
                    }
                }
            }

            #[automatically_derived]
            impl<__S> ::arcane::es::event::Sourced<Event> for Option<__S>
            where
                Self: ::arcane::es::event::Sourced<
                          ::arcane::es::event::Initial<FileEvent>
                      > +
                      ::arcane::es::event::Sourced<FileEvent>
            {
                fn apply(&mut self, event: &Event) {
                    match event {
                        Event::File(f) => {
                            ::arcane::es::event::Sourced::apply(
                                self,
                                <::arcane::es::event::Initial<FileEvent>
                                 as ::arcane::RefCast>::ref_cast(f)
                            );
                        },
                        Event::DupFile(f) => {
                            ::arcane::es::event::Sourced::apply(self, f);
                        },
                    }
                }
            }

            #[automatically_derived]
            impl<'__raw, __Data> ::std::convert::TryFrom<Event>
                             for ::arcane::es::event::Raw<'__raw, __Data, ()>
            where
                __Data: ::std::convert::TryFrom<FileEvent>
            {
                type Error = <
                    __Data as ::std::convert::TryFrom<FileEvent>
                >::Error;

                fn try_from(event: Event)
                    -> ::std::result::Result<Self, Self::Error>
                {
                    Ok(Self {
                        name: ::std::borrow::Cow::from(<
                            Event as ::arcane::es::Event
                        >::name(&event)),
                        revision: (),
                        data: match event {
                            Event::File(ev) => <
                                __Data as ::std::convert::TryFrom<FileEvent>
                            >::try_from(ev)?,
                            Event::DupFile(ev) => <
                                __Data as ::std::convert::TryFrom<FileEvent>
                            >::try_from(ev)?,
                        },
                    })
                }
            }

            #[automatically_derived]
            impl<'__raw, __Data> ::std::convert::TryFrom<
                ::arcane::es::event::Raw<'__raw, __Data, ()>
            > for Event
            where
                FileEvent: ::std::convert::TryFrom<__Data>
            {
                type Error = ::arcane::es::event::FromRawError<
                    <FileEvent as TryFrom<__Data>>::Error,
                    ()
                >;

                fn try_from(
                    raw: ::arcane::es::event::Raw<'__raw, __Data, ()>
                ) -> ::std::result::Result<Self, Self::Error> {
                    for (_, var_name, var_rev) in <
                        FileEvent as ::arcane::es::event::codegen::Reflect
                    >::META {
                        if *var_name == raw.name {
                            return <
                                FileEvent as ::std::convert::TryFrom<__Data>
                            >::try_from(raw.data)
                            .map(Self::File)
                            .map_err(
                                ::arcane::es::event::FromRawError::FromDataError
                            );
                        }
                    }

                    Err(::arcane::es::event::FromRawError::UnknownEvent {
                        name: raw.name.to_string(),
                        revision: raw.revision,
                    })
                }
            }

            #[automatically_derived]
            #[doc(hidden)]
            impl ::arcane::es::event::codegen::Reflect for Event {
                #[doc(hidden)]
                const META: &'static [
                    (&'static str, &'static str, &'static str)
                ] = {
                    ::arcane::es::event::codegen::const_concat_slices!(
                        <FileEvent
                         as ::arcane::es::event::codegen::Reflect>::META,
                        <FileEvent
                         as ::arcane::es::event::codegen::Reflect>::META,
                    )
                };
            }

            #[automatically_derived]
            #[doc(hidden)]
            const _: () = ::std::assert!(
                !::arcane::es::event::codegen
                 ::has_different_types_with_same_name_and_revision
                 ::<Event<> >(),
                "having different `Event` types with the same name \
                 and revision inside a single enum is forbidden",
            );
        };
        if cfg!(feature = "reflect") {
            output.extend([quote! {
                #[automatically_derived]
                impl ::arcane::es::event::reflect::Static for Event {
                    const NAMES: &'static [::arcane::es::event::Name] = {
                        ::arcane::es::event::codegen::const_concat_slices!(
                            <FileEvent
                             as ::arcane::es::event::reflect::Static>::NAMES,
                            <FileEvent
                             as ::arcane::es::event::reflect::Static>::NAMES,
                        )
                    };
                }
            }]);
        }

        assert_eq!(derive(input).unwrap().to_string(), output.to_string());
    }

    #[allow(clippy::too_many_lines)]
    #[test]
    fn derives_enum_impl_with_revision() {
        let input = parse_quote! {
            #[event(revision)]
            enum Event {
                #[event(init)]
                File(FileEvent),
                Chat(ChatEvent),
            }
        };

        let mut output = quote! {
            #[automatically_derived]
            impl ::arcane::es::Event for Event {
                fn name(&self) -> ::arcane::es::event::Name {
                    match self {
                        Self::File(f) => ::arcane::es::Event::name(f),
                        Self::Chat(f) => ::arcane::es::Event::name(f),
                    }
                }
            }

            #[automatically_derived]
            impl ::arcane::es::event::Revisable for Event
            where
                FileEvent: ::arcane::es::event::Revisable,
                ::arcane::es::event::RevisionOf<FileEvent>: From<
                    ::arcane::es::event::RevisionOf<FileEvent>
                >,
                ChatEvent: ::arcane::es::event::Revisable,
                ::arcane::es::event::RevisionOf<FileEvent>: From<
                    ::arcane::es::event::RevisionOf<ChatEvent>
                >
            {
                type Revision = ::arcane::es::event::RevisionOf<FileEvent>;

                fn revision(&self) -> Self::Revision {
                    match self {
                        Self::File(f) => Self::Revision::from(
                            ::arcane::es::event::Revisable::revision(f)
                        ),
                        Self::Chat(f) => Self::Revision::from(
                            ::arcane::es::event::Revisable::revision(f)
                        ),
                    }
                }
            }

            #[automatically_derived]
            impl<__S> ::arcane::es::event::Sourced<Event> for Option<__S>
            where
                Self: ::arcane::es::event::Sourced<
                          ::arcane::es::event::Initial<FileEvent>
                      > +
                      ::arcane::es::event::Sourced<ChatEvent>
            {
                fn apply(&mut self, event: &Event) {
                    match event {
                        Event::File(f) => {
                            ::arcane::es::event::Sourced::apply(
                                self,
                                <::arcane::es::event::Initial<FileEvent>
                                 as ::arcane::RefCast>::ref_cast(f)
                            );
                        },
                        Event::Chat(f) => {
                            ::arcane::es::event::Sourced::apply(self, f);
                        },
                    }
                }
            }

            #[automatically_derived]
            impl<'__raw, __Data> ::std::convert::TryFrom<Event>
                             for ::arcane::es::event::Raw<
                                 '__raw,
                                 __Data,
                                 ::arcane::es::event::RevisionOf<Event>
                             >
            where
                __Data: ::std::convert::TryFrom<FileEvent>,
                __Data: ::std::convert::TryFrom<
                    ChatEvent,
                    Error = <
                        __Data as ::std::convert::TryFrom<FileEvent>
                    >::Error
                >
            {
                type Error = <
                    __Data as ::std::convert::TryFrom<FileEvent>
                >::Error;

                fn try_from(event: Event)
                    -> ::std::result::Result<Self, Self::Error>
                {
                    Ok(Self {
                        name: ::std::borrow::Cow::from(<
                            Event as ::arcane::es::Event
                        >::name(&event)),
                        revision: <
                            Event as ::arcane::es::event::Revisable
                        >::revision(&event),
                        data: match event {
                            Event::File(ev) => <
                                __Data as ::std::convert::TryFrom<FileEvent>
                            >::try_from(ev)?,
                            Event::Chat(ev) => <
                                __Data as ::std::convert::TryFrom<ChatEvent>
                            >::try_from(ev)?,
                        },
                    })
                }
            }

            #[automatically_derived]
            impl<'__raw, __Data> ::std::convert::TryFrom<
                ::arcane::es::event::Raw<
                    '__raw,
                    __Data,
                    ::arcane::es::event::RevisionOf<Event>
                >
            > for Event
            where
                FileEvent: ::std::convert::TryFrom<__Data>,
                ChatEvent: ::std::convert::TryFrom<
                    __Data,
                    Error = <
                        FileEvent as ::std::convert::TryFrom<__Data>
                    >::Error
                >
            {
                type Error = ::arcane::es::event::FromRawError<
                    <FileEvent as TryFrom<__Data>>::Error,
                    ::arcane::es::event::RevisionOf<Event>
                >;

                fn try_from(
                    raw: ::arcane::es::event::Raw<
                        '__raw,
                        __Data,
                        ::arcane::es::event::RevisionOf<Event>
                    >
                ) -> ::std::result::Result<Self, Self::Error> {
                    for (_, var_name, var_rev) in <
                        FileEvent as ::arcane::es::event::codegen::Reflect
                    >::META {
                        if *var_name == raw.name && *var_rev == <
                            ::arcane::es::event::RevisionOf<Event>
                            as ::std::string::ToString
                        >::to_string(&raw.revision) {
                            return <
                                FileEvent as ::std::convert::TryFrom<__Data>
                            >::try_from(raw.data)
                            .map(Self::File)
                            .map_err(
                                ::arcane::es::event::FromRawError::FromDataError
                            );
                        }
                    }

                    for (_, var_name, var_rev) in <
                        ChatEvent as ::arcane::es::event::codegen::Reflect
                    >::META {
                        if *var_name == raw.name && *var_rev == <
                            ::arcane::es::event::RevisionOf<Event>
                            as ::std::string::ToString
                        >::to_string(&raw.revision) {
                            return <
                                ChatEvent as ::std::convert::TryFrom<__Data>
                            >::try_from(raw.data)
                            .map(Self::Chat)
                            .map_err(
                                ::arcane::es::event::FromRawError::FromDataError
                            );
                        }
                    }

                    Err(::arcane::es::event::FromRawError::UnknownEvent {
                        name: raw.name.to_string(),
                        revision: raw.revision,
                    })
                }
            }

            #[automatically_derived]
            #[doc(hidden)]
            impl ::arcane::es::event::codegen::Reflect for Event {
                #[doc(hidden)]
                const META: &'static [
                    (&'static str, &'static str, &'static str)
                ] = {
                    ::arcane::es::event::codegen::const_concat_slices!(
                        <FileEvent
                         as ::arcane::es::event::codegen::Reflect>::META,
                        <ChatEvent
                         as ::arcane::es::event::codegen::Reflect>::META,
                    )
                };
            }

            #[automatically_derived]
            #[doc(hidden)]
            const _: () = ::std::assert!(
                !::arcane::es::event::codegen
                 ::has_different_types_with_same_name_and_revision
                 ::<Event<> >(),
                "having different `Event` types with the same name \
                 and revision inside a single enum is forbidden",
            );
        };
        if cfg!(feature = "reflect") {
            output.extend([quote! {
                #[automatically_derived]
                impl ::arcane::es::event::reflect::Static for Event {
                    const NAMES: &'static [::arcane::es::event::Name] = {
                        ::arcane::es::event::codegen::const_concat_slices!(
                            <FileEvent
                             as ::arcane::es::event::reflect::Static>::NAMES,
                            <ChatEvent
                             as ::arcane::es::event::reflect::Static>::NAMES,
                        )
                    };
                }

                #[automatically_derived]
                impl ::arcane::es::event::reflect::Concrete for Event {
                    const REVISIONS: &'static [::arcane::es::event::Version] = {
                        ::arcane::es::event::codegen::const_concat_slices!(
                            <FileEvent as
                             ::arcane::es::event::reflect::Concrete>::REVISIONS,
                            <ChatEvent as
                             ::arcane::es::event::reflect::Concrete>::REVISIONS,
                        )
                    };
                }
            }]);
        }

        assert_eq!(derive(input).unwrap().to_string(), output.to_string());
    }

    #[expect(clippy::too_many_lines, reason = "OK for macro expansion test")]
    #[test]
    fn derives_enum_with_generics_impl() {
        let input = parse_quote! {
            #[event(revision)]
            enum Event<'a, F, C> {
                #[event(init)]
                File(FileEvent<'a, F>),
                Chat(ChatEvent<'a, C>),
            }
        };

        let mut output = quote! {
            #[automatically_derived]
            impl<'a, F, C> ::arcane::es::Event for Event<'a, F, C> {
                fn name(&self) -> ::arcane::es::event::Name {
                    match self {
                        Self::File(f) => ::arcane::es::Event::name(f),
                        Self::Chat(f) => ::arcane::es::Event::name(f),
                    }
                }
            }

            #[automatically_derived]
            impl<'a, F, C> ::arcane::es::event::Revisable for Event<'a, F, C>
            where
                FileEvent<'a, F>: ::arcane::es::event::Revisable,
                ::arcane::es::event::RevisionOf<FileEvent<'a, F> >: From<
                    ::arcane::es::event::RevisionOf<FileEvent<'a, F> >
                >,
                ChatEvent<'a, C>: ::arcane::es::event::Revisable,
                ::arcane::es::event::RevisionOf<FileEvent<'a, F> >: From<
                    ::arcane::es::event::RevisionOf<ChatEvent<'a, C> >
                >
            {
                type Revision = ::arcane::es::event::RevisionOf<
                    FileEvent<'a, F>
                >;

                fn revision(&self) -> Self::Revision {
                    match self {
                        Self::File(f) => Self::Revision::from(
                            ::arcane::es::event::Revisable::revision(f)
                        ),
                        Self::Chat(f) => Self::Revision::from(
                            ::arcane::es::event::Revisable::revision(f)
                        ),
                    }
                }
            }

            #[automatically_derived]
            impl<'a, F, C, __S> ::arcane::es::event::Sourced<Event<'a, F, C> >
             for Option<__S>
            where
                Self: ::arcane::es::event::Sourced<
                          ::arcane::es::event::Initial<FileEvent<'a, F> >
                      > +
                      ::arcane::es::event::Sourced<ChatEvent<'a, C> >
            {
                fn apply(&mut self, event: &Event<'a, F, C>) {
                    match event {
                        Event::<'a, F, C>::File(f) => {
                            ::arcane::es::event::Sourced::apply(
                                self,
                                <::arcane::es::event::Initial<FileEvent<'a, F> >
                                 as ::arcane::RefCast>::ref_cast(f)
                            );
                        },
                        Event::<'a, F, C>::Chat(f) => {
                            ::arcane::es::event::Sourced::apply(self, f);
                        },
                    }
                }
            }

            #[automatically_derived]
            impl<'a, '__raw, F, C, __Data> ::std::convert::TryFrom<
                Event<'a, F, C>
            > for ::arcane::es::event::Raw<
                '__raw,
                __Data,
                ::arcane::es::event::RevisionOf< Event<'a, F, C> >
            >
            where
                __Data: ::std::convert::TryFrom< FileEvent<'a, F> >,
                __Data: ::std::convert::TryFrom<
                    ChatEvent<'a, C>,
                    Error = <
                        __Data as ::std::convert::TryFrom< FileEvent<'a, F> >
                    >::Error
                >
            {
                type Error = <
                    __Data as ::std::convert::TryFrom< FileEvent<'a, F> >
                >::Error;

                fn try_from(event: Event<'a, F, C>)
                    -> ::std::result::Result<Self, Self::Error>
                {
                    Ok(Self {
                        name: ::std::borrow::Cow::from(<
                            Event<'a, F, C> as ::arcane::es::Event
                        >::name(&event)),
                        revision: <
                            Event<'a, F, C> as ::arcane::es::event::Revisable
                        >::revision(&event),
                        data: match event {
                            Event::File(ev) => <
                                __Data as ::std::convert::TryFrom<
                                    FileEvent<'a, F>
                                >
                            >::try_from(ev)?,
                            Event::Chat(ev) => <
                                __Data as ::std::convert::TryFrom<
                                    ChatEvent<'a, C>
                                >
                            >::try_from(ev)?,
                        },
                    })
                }
            }

            #[automatically_derived]
            impl<'a, '__raw, F, C, __Data> ::std::convert::TryFrom<
                ::arcane::es::event::Raw<
                    '__raw,
                    __Data,
                    ::arcane::es::event::RevisionOf< Event<'a, F, C> >
                >
            > for Event<'a, F, C>
            where
                FileEvent<'a, F>: ::std::convert::TryFrom<__Data>,
                ChatEvent<'a, C>: ::std::convert::TryFrom<
                    __Data,
                    Error = <
                        FileEvent<'a, F> as ::std::convert::TryFrom<__Data>
                    >::Error
                >
            {
                type Error = ::arcane::es::event::FromRawError<
                    <FileEvent<'a, F> as TryFrom<__Data>>::Error,
                    ::arcane::es::event::RevisionOf< Event<'a, F, C> >
                >;

                fn try_from(
                    raw: ::arcane::es::event::Raw<
                        '__raw,
                        __Data,
                        ::arcane::es::event::RevisionOf< Event<'a, F, C> >
                    >
                ) -> ::std::result::Result<Self, Self::Error> {
                    for (_, var_name, var_rev) in <
                        FileEvent<'a, F>
                        as ::arcane::es::event::codegen::Reflect
                    >::META {
                        if *var_name == raw.name && *var_rev == <
                            ::arcane::es::event::RevisionOf< Event<'a, F, C> >
                            as ::std::string::ToString
                        >::to_string(&raw.revision) {
                            return <
                                FileEvent<'a, F>
                                as ::std::convert::TryFrom<__Data>
                            >::try_from(raw.data)
                            .map(Self::File)
                            .map_err(
                                ::arcane::es::event::FromRawError::FromDataError
                            );
                        }
                    }

                    for (_, var_name, var_rev) in <
                        ChatEvent<'a, C>
                        as ::arcane::es::event::codegen::Reflect
                    >::META {
                        if *var_name == raw.name && *var_rev == <
                            ::arcane::es::event::RevisionOf< Event<'a, F, C> >
                            as ::std::string::ToString
                        >::to_string(&raw.revision) {
                            return <
                                ChatEvent<'a, C>
                                as ::std::convert::TryFrom<__Data>
                            >::try_from(raw.data)
                            .map(Self::Chat)
                            .map_err(
                                ::arcane::es::event::FromRawError::FromDataError
                            );
                        }
                    }

                    Err(::arcane::es::event::FromRawError::UnknownEvent {
                        name: raw.name.to_string(),
                        revision: raw.revision,
                    })
                }
            }

            #[automatically_derived]
            #[doc(hidden)]
            impl<'a, F, C> ::arcane::es::event::codegen::Reflect
             for Event<'a, F, C>
            {
                #[doc(hidden)]
                const META: &'static [
                    (&'static str, &'static str, &'static str)
                ] = {
                    type F = ();
                    type C = ();

                    ::arcane::es::event::codegen::const_concat_slices!(
                        <FileEvent<'a, F>
                         as ::arcane::es::event::codegen::Reflect>::META,
                        <ChatEvent<'a, C>
                         as ::arcane::es::event::codegen::Reflect>::META,
                    )
                };
            }

            #[automatically_derived]
            #[doc(hidden)]
            const _: () = ::std::assert!(
                !::arcane::es::event::codegen
                 ::has_different_types_with_same_name_and_revision
                 ::<Event<'static, (), ()> >(),
                "having different `Event` types with the same name \
                 and revision inside a single enum is forbidden",
            );
        };
        if cfg!(feature = "reflect") {
            output.extend([quote! {
                #[automatically_derived]
                impl<'a, F, C> ::arcane::es::event::reflect::Static
                 for Event<'a, F, C>
                {
                    const NAMES: &'static [::arcane::es::event::Name] = {
                        type F = ();
                        type C = ();

                        ::arcane::es::event::codegen::const_concat_slices!(
                            <FileEvent<'a, F>
                             as ::arcane::es::event::reflect::Static>::NAMES,
                            <ChatEvent<'a, C>
                             as ::arcane::es::event::reflect::Static>::NAMES,
                        )
                    };
                }

                #[automatically_derived]
                impl<'a, F, C> ::arcane::es::event::reflect::Concrete
                 for Event<'a, F, C>
                {
                    const REVISIONS: &'static [::arcane::es::event::Version] = {
                        type F = ();
                        type C = ();

                        ::arcane::es::event::codegen::const_concat_slices!(
                            <FileEvent<'a, F> as
                             ::arcane::es::event::reflect::Concrete>::REVISIONS,
                            <ChatEvent<'a, C> as
                             ::arcane::es::event::reflect::Concrete>::REVISIONS,
                        )
                    };
                }
            }]);
        }

        assert_eq!(derive(input).unwrap().to_string(), output.to_string());
    }

    #[expect(clippy::too_many_lines, reason = "OK for macro expansion test")]
    #[test]
    fn ignores_ignored_variant() {
        let input_ignore = parse_quote! {
            #[event(revision)]
            enum Event {
                File(FileEvent),
                Chat(ChatEvent),
                #[event(ignore)]
                _NonExhaustive,
            }
        };
        let input_skip = parse_quote! {
            #[event(revision)]
            enum Event {
                File(FileEvent),
                Chat(ChatEvent),
                #[event(skip)]
                _NonExhaustive,
            }
        };

        let mut output = quote! {
            #[automatically_derived]
            impl ::arcane::es::Event for Event {
                fn name(&self) -> ::arcane::es::event::Name {
                    match self {
                        Self::File(f) => ::arcane::es::Event::name(f),
                        Self::Chat(f) => ::arcane::es::Event::name(f),
                        _ => unreachable!(),
                    }
                }
            }

            #[automatically_derived]
            impl ::arcane::es::event::Revisable for Event
            where
                FileEvent: ::arcane::es::event::Revisable,
                ::arcane::es::event::RevisionOf<FileEvent>: From<
                    ::arcane::es::event::RevisionOf<FileEvent>
                >,
                ChatEvent: ::arcane::es::event::Revisable,
                ::arcane::es::event::RevisionOf<FileEvent>: From<
                    ::arcane::es::event::RevisionOf<ChatEvent>
                >
            {
                type Revision = ::arcane::es::event::RevisionOf<FileEvent>;

                fn revision(&self) -> Self::Revision {
                    match self {
                        Self::File(f) => Self::Revision::from(
                            ::arcane::es::event::Revisable::revision(f)
                        ),
                        Self::Chat(f) => Self::Revision::from(
                            ::arcane::es::event::Revisable::revision(f)
                        ),
                        _ => unreachable!(),
                    }
                }
            }

            #[automatically_derived]
            impl<__S> ::arcane::es::event::Sourced<Event> for Option<__S>
            where
                Self: ::arcane::es::event::Sourced<FileEvent> +
                      ::arcane::es::event::Sourced<ChatEvent>
            {
                fn apply(&mut self, event: &Event) {
                    match event {
                        Event::File(f) => {
                            ::arcane::es::event::Sourced::apply(self, f);
                        },
                        Event::Chat(f) => {
                            ::arcane::es::event::Sourced::apply(self, f);
                        },
                        _ => unreachable!(),
                    }
                }
            }

            #[automatically_derived]
            impl<'__raw, __Data> ::std::convert::TryFrom<Event>
                             for ::arcane::es::event::Raw<
                                 '__raw,
                                 __Data,
                                 ::arcane::es::event::RevisionOf<Event>
                             >
            where
                __Data: ::std::convert::TryFrom<FileEvent>,
                __Data: ::std::convert::TryFrom<
                    ChatEvent,
                    Error = <
                        __Data as ::std::convert::TryFrom<FileEvent>
                    >::Error
                >
            {
                type Error = <
                    __Data as ::std::convert::TryFrom<FileEvent>
                >::Error;

                fn try_from(event: Event)
                    -> ::std::result::Result<Self, Self::Error>
                {
                    Ok(Self {
                        name: ::std::borrow::Cow::from(<
                            Event as ::arcane::es::Event
                        >::name(&event)),
                        revision: <
                            Event as ::arcane::es::event::Revisable
                        >::revision(&event),
                        data: match event {
                            Event::File(ev) => <
                                __Data as ::std::convert::TryFrom<FileEvent>
                            >::try_from(ev)?,
                            Event::Chat(ev) => <
                                __Data as ::std::convert::TryFrom<ChatEvent>
                            >::try_from(ev)?,
                            _ => unreachable!(),
                        },
                    })
                }
            }

            #[automatically_derived]
            impl<'__raw, __Data> ::std::convert::TryFrom<
                ::arcane::es::event::Raw<
                    '__raw,
                    __Data,
                    ::arcane::es::event::RevisionOf<Event>
                >
            > for Event
            where
                FileEvent: ::std::convert::TryFrom<__Data>,
                ChatEvent: ::std::convert::TryFrom<
                    __Data,
                    Error = <
                        FileEvent as ::std::convert::TryFrom<__Data>
                    >::Error
                >
            {
                type Error = ::arcane::es::event::FromRawError<
                    <FileEvent as TryFrom<__Data>>::Error,
                    ::arcane::es::event::RevisionOf<Event>
                >;

                fn try_from(
                    raw: ::arcane::es::event::Raw<
                        '__raw,
                        __Data,
                        ::arcane::es::event::RevisionOf<Event>
                    >
                ) -> ::std::result::Result<Self, Self::Error> {
                    for (_, var_name, var_rev) in <
                        FileEvent as ::arcane::es::event::codegen::Reflect
                    >::META {
                        if *var_name == raw.name && *var_rev == <
                            ::arcane::es::event::RevisionOf<Event>
                            as ::std::string::ToString
                        >::to_string(&raw.revision) {
                            return <
                                FileEvent as ::std::convert::TryFrom<__Data>
                            >::try_from(raw.data)
                            .map(Self::File)
                            .map_err(
                                ::arcane::es::event::FromRawError::FromDataError
                            );
                        }
                    }

                    for (_, var_name, var_rev) in <
                        ChatEvent as ::arcane::es::event::codegen::Reflect
                    >::META {
                        if *var_name == raw.name && *var_rev == <
                            ::arcane::es::event::RevisionOf<Event>
                            as ::std::string::ToString
                        >::to_string(&raw.revision) {
                            return <
                                ChatEvent as ::std::convert::TryFrom<__Data>
                            >::try_from(raw.data)
                            .map(Self::Chat)
                            .map_err(
                                ::arcane::es::event::FromRawError::FromDataError
                            );
                        }
                    }

                    Err(::arcane::es::event::FromRawError::UnknownEvent {
                        name: raw.name.to_string(),
                        revision: raw.revision,
                    })
                }
            }

            #[automatically_derived]
            #[doc(hidden)]
            impl ::arcane::es::event::codegen::Reflect for Event {
                #[doc(hidden)]
                const META: &'static [
                    (&'static str, &'static str, &'static str)
                ] = {
                    ::arcane::es::event::codegen::const_concat_slices!(
                        <FileEvent
                         as ::arcane::es::event::codegen::Reflect>::META,
                        <ChatEvent
                         as ::arcane::es::event::codegen::Reflect>::META,
                    )
                };
            }

            #[automatically_derived]
            #[doc(hidden)]
            const _: () = ::std::assert!(
                !::arcane::es::event::codegen
                 ::has_different_types_with_same_name_and_revision
                 ::<Event<> >(),
                "having different `Event` types with the same name \
                 and revision inside a single enum is forbidden",
            );
        };
        if cfg!(feature = "reflect") {
            output.extend([quote! {
                #[automatically_derived]
                impl ::arcane::es::event::reflect::Static for Event {
                    const NAMES: &'static [::arcane::es::event::Name] = {
                        ::arcane::es::event::codegen::const_concat_slices!(
                            <FileEvent
                             as ::arcane::es::event::reflect::Static>::NAMES,
                            <ChatEvent
                             as ::arcane::es::event::reflect::Static>::NAMES,
                        )
                    };
                }

                #[automatically_derived]
                impl ::arcane::es::event::reflect::Concrete for Event {
                    const REVISIONS: &'static [::arcane::es::event::Version] = {
                        ::arcane::es::event::codegen::const_concat_slices!(
                            <FileEvent as
                             ::arcane::es::event::reflect::Concrete>::REVISIONS,
                            <ChatEvent as
                             ::arcane::es::event::reflect::Concrete>::REVISIONS,
                        )
                    };
                }
            }]);
        }

        let input_ignore = derive(input_ignore).unwrap().to_string();
        let input_skip = derive(input_skip).unwrap().to_string();

        assert_eq!(input_ignore, output.to_string());
        assert_eq!(input_skip, input_ignore);
    }

    #[test]
    fn errors_on_multiple_fields_in_variant() {
        let input = parse_quote! {
            enum Event {
                Event1(Event1),
                Event2 {
                    event: Event2,
                    second_field: Event3,
                }
            }
        };

        let err = derive(input).unwrap_err();

        assert_eq!(err.to_string(), "enum variants must have exactly 1 field");
    }

    #[test]
    fn errors_on_struct() {
        let input = parse_quote! {
            struct Event;
        };

        let err = derive(input).unwrap_err();

        assert_eq!(err.to_string(), "only enums are allowed");
    }

    #[test]
    fn errors_on_empty_enum() {
        let input = parse_quote! {
            enum Event {}
        };

        let err = derive(input).unwrap_err();

        assert_eq!(
            err.to_string(),
            "enum must have at least one non-ignored variant",
        );
    }

    #[test]
    fn errors_on_enum_with_ignored_variant_only() {
        let input = parse_quote! {
            enum Event {
                #[event(ignore)]
                _NonExhaustive,
            }
        };

        let err = derive(input).unwrap_err();

        assert_eq!(
            err.to_string(),
            "enum must have at least one non-ignored variant",
        );
    }

    #[test]
    fn errors_on_both_init_and_ignored_variant() {
        let input = parse_quote! {
            enum Event {
                #[event(init, ignore)]
                Event1(Event1),
            }
        };

        let err = derive(input).unwrap_err();

        assert_eq!(
            err.to_string(),
            "`init` and `ignore`/`skip` arguments are mutually exclusive",
        );
    }
}
