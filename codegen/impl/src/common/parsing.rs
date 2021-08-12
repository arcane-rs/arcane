/// Common errors of parsing Rust attributes, appeared in this crate.
pub mod err {
    use proc_macro2::Span;
    use syn::spanned::Spanned;

    /// Creates a "duplicated argument" [`syn::Error`] pointing to the given
    /// `span`.
    #[inline]
    #[must_use]
    pub fn dup_attr_arg<S: AsSpan>(span: S) -> syn::Error {
        syn::Error::new(span.as_span(), "duplicated attribute argument found")
    }

    /// Creates an "unknown argument" [`syn::Error`] for the given `name`
    /// pointing to the given `span`.
    #[must_use]
    pub fn unknown_attr_arg<S: AsSpan>(span: S, name: &str) -> syn::Error {
        syn::Error::new(
            span.as_span(),
            format!("unknown `{}` attribute argument", name),
        )
    }

    /// Helper coercion for [`Span`] and [`Spanned`] types to use in function
    /// arguments.
    pub trait AsSpan {
        /// Returns the coerced [`Span`].
        #[must_use]
        fn as_span(&self) -> Span;
    }

    impl AsSpan for Span {
        #[inline]
        fn as_span(&self) -> Self {
            *self
        }
    }

    impl<T: Spanned> AsSpan for &T {
        #[inline]
        fn as_span(&self) -> Span {
            self.span()
        }
    }
}
