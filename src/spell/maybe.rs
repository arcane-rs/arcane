use std::{marker::PhantomData, sync::atomic::AtomicPtr, fmt};

use derive_more::{Deref, DerefMut};

pub trait Maybe<T> {
    #[must_use]
    fn into_option(self) -> Option<T>;

    #[must_use]
    fn as_option(&self) -> Option<&T>;
}

impl<T> Maybe<T> for Option<T> {
    #[inline]
    fn into_option(self) -> Self {
        self
    }

    #[inline]
    fn as_option(&self) -> Option<&T> {
        self.as_ref()
    }
}

pub struct Nothing<T: ?Sized>(PhantomData<AtomicPtr<Box<T>>>);

impl<T: ?Sized> Nothing<T> {
    #[inline]
    #[must_use]
    pub fn here() -> Self {
        Self(PhantomData)
    }
}

impl<T: ?Sized> Clone for Nothing<T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Copy for Nothing<T> {}

impl<T: ?Sized> fmt::Debug for Nothing<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Nothing").field(&self.0).finish()
    }
}

impl<T> Maybe<T> for Nothing<T> {
    #[inline]
    fn into_option(self) -> Option<T> {
        None
    }

    #[inline]
    fn as_option(&self) -> Option<&T> {
        None
    }
}

#[derive(Clone, Copy, Debug, Deref, DerefMut)]
pub struct Just<T: ?Sized>(pub T);

impl<T> Maybe<T> for Just<T> {
    #[inline]
    fn into_option(self) -> Option<T> {
        Some(self.0)
    }

    #[inline]
    fn as_option(&self) -> Option<&T> {
        Some(&self.0)
    }
}
