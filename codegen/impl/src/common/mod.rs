pub mod parsing;

/// Handy extension of [`Option`] methods, used in this crate.
pub trait OptionExt {
    /// Type of the value wrapped into this [`Option`].
    type Inner;

    /// Transforms the `Option<T>` into a `Result<(), E>`, mapping `None` to
    /// `Ok(())` and `Some(v)` to `Err(err(v))`.
    ///
    /// # Errors
    ///
    /// If `self` is [`None`].
    fn none_or_else<E, F>(self, err: F) -> Result<(), E>
    where
        F: FnOnce(Self::Inner) -> E;
}

impl<T> OptionExt for Option<T> {
    type Inner = T;

    #[inline]
    fn none_or_else<E, F>(self, err: F) -> Result<(), E>
    where
        F: FnOnce(T) -> E,
    {
        match self {
            Some(v) => Err(err(v)),
            None => Ok(()),
        }
    }
}
