//! [`Variant`] definition.

use syn::spanned::Spanned as _;
use synthez::ParseAttrs;

/// Attributes of `#[derive(Event)]` macro placed on a [`Variant`].
#[derive(Debug, Default, ParseAttrs)]
pub struct Attrs {
    /// Indicator whether this enum variant should be used as
    /// [`event::Initialized`] rather than [`event::Sourced`].
    ///
    /// [`event::Initialized`]: arcane_core::es::event::Initialized
    /// [`event::Sourced`]: arcane_core::es::event::Sourced
    #[parse(ident, alias = initial)]
    pub init: Option<syn::Ident>,

    /// Indicator whether to ignore this enum variant for code generation.
    #[parse(ident, alias = skip)]
    pub ignore: Option<syn::Ident>,
}

/// Type of event sourcing the [`Variant`] is using.
#[derive(Clone, Copy, Debug)]
pub enum Sourcing {
    /// [`Variant`] used as [`event::Initialized`].
    ///
    /// [`event::Initialized`]: arcane_core::es::event::Initialized
    Initialized,

    /// [`Variant`] used as [`event::Sourced`].
    ///
    /// [`event::Sourced`]: arcane_core::es::event::Sourced
    Sourced,
}

/// Single-fielded variant of the enum deriving `#[derive(Event)]`.
#[derive(Debug)]
pub struct Variant {
    /// [`syn::Ident`](struct@syn::Ident) of this [`Variant`].
    pub ident: syn::Ident,

    /// [`syn::Type`] of this [`Variant`].
    pub ty: syn::Type,

    /// [`Sourcing`] type of this [`Variant`].
    pub sourcing: Sourcing,
}

impl Variant {
    /// Validates the given [`syn::Variant`], parses its [`Attrs`] and returns
    /// a [`Variant`] if the validation succeeds.
    ///
    /// # Errors
    ///
    /// - If [`Attrs`] failed to parse.
    /// - If [`Attrs::init`] and [`Attrs::ignore`] were specified
    ///   simultaneously.
    /// - If [`syn::Variant`] doesn't have exactly one unnamed 1 [`syn::Field`]
    ///   and is not ignored.
    #[allow(
        clippy::missing_panics_doc,
        clippy::unwrap_in_result,
        clippy::unwrap_used
    )]
    pub fn parse(variant: &syn::Variant) -> syn::Result<Option<Self>> {
        let attrs = Attrs::parse_attrs("event", variant)?;

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

        // PANIC: Unwrap is OK here, because we've already checked that
        //        `variant.fields` has exactly 1 unnamed field.
        let field = variant.fields.iter().next().unwrap();
        let sourcing = attrs
            .init
            .map_or(Sourcing::Sourced, |_| Sourcing::Initialized);

        Ok(Some(Self {
            ident: variant.ident.clone(),
            ty: field.ty.clone(),
            sourcing,
        }))
    }
}
