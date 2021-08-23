//! [Event Sourcing] related definitions.
//!
//! [Event Sourcing]: https://martinfowler.com/eaaDev/EventSourcing.html

use std::{convert::TryFrom, num::NonZeroU16};

use derive_more::{Display, Into};
use ref_cast::RefCast;

/// [Event Sourcing] event that describes something that has occurred (happened
/// fact).
///
/// [Event Sourcing]: https://martinfowler.com/eaaDev/EventSourcing.html
pub trait Event {
    /// Returns [`Name`] of this [`Event`].
    ///
    /// _Note:_ This should effectively be a constant value, and should never
    /// change.
    #[must_use]
    fn name(&self) -> Name;

    /// Returns [`Version`] of this [`Event`].
    #[must_use]
    fn ver(&self) -> Version;
}

/// Versioned [`Event`].
///
/// The single type of [`Event`] may have different versions, which allows
/// evolving [`Event`] in the type. To overcome the necessity of dealing with
/// multiple types of the same [`Event`], it's recommended for the last actual
/// version of [`Event`] to implement trait [`From`] its previous versions, so
/// they can be automatically transformed into the latest actual version of
pub trait Versioned {
    /// Returns [`Name`] of this [`Event`].
    ///
    /// _Note:_ This should effectively be a constant value, and should never
    /// change.
    #[must_use]
    fn name() -> Name;

    /// Returns [`Version`] of this [`Event`].
    #[must_use]
    fn ver() -> Version;
}

/// Fully qualified name of an [`Event`].
pub type Name = &'static str;

/// Revision number of an [`Event`].
#[derive(
    Clone, Copy, Debug, Display, Eq, Hash, Into, Ord, PartialEq, PartialOrd,
)]
pub struct Version(NonZeroU16);

impl Version {
    /// Creates a new [`Version`] out of the given `val`ue.
    ///
    /// The given value should not be `0` (zero) and fit into [`u16`] size.
    #[inline]
    #[must_use]
    pub fn try_new<N>(val: N) -> Option<Self>
    where
        u16: TryFrom<N>,
    {
        Some(Self(NonZeroU16::new(u16::try_from(val).ok()?)?))
    }

    /// Creates a new [`Version`] out of the given `val`ue without checking its
    /// invariants.
    ///
    /// # Safety
    ///
    /// The given `val`ue must not be `0` (zero).
    #[allow(unsafe_code)]
    #[inline]
    #[must_use]
    pub const unsafe fn new_unchecked(val: u16) -> Self {
        Self(NonZeroU16::new_unchecked(val))
    }
}

impl<Ev: Versioned + ?Sized> Event for Ev {
    fn name(&self) -> Name {
        <Self as Versioned>::name()
    }

    fn ver(&self) -> Version {
        <Self as Versioned>::ver()
    }
}

/// State that can be calculated by applying specified [`Event`].
pub trait Sourced<Ev: ?Sized> {
    /// Applies given [`Event`] to the current state.
    fn apply(&mut self, event: &Ev);
}

impl<Ev: Event + ?Sized, S: Sourced<Ev>> Sourced<Ev> for Option<S> {
    fn apply(&mut self, event: &Ev) {
        if let Some(state) = self {
            state.apply(event);
        }
    }
}

/// Before items can be [`Sourced`], they need to be [`Initialized`].
pub trait Initialized<Ev: ?Sized> {
    /// Creates initial state from given [`Event`].
    fn init(event: &Ev) -> Self;
}

/// Wrapper-type intended for [`Event`]s that can initialize [`Sourced`] items.
#[derive(Clone, Copy, Debug, Display, RefCast)]
#[repr(transparent)]
pub struct Initial<Ev: ?Sized>(pub Ev);

impl<Ev: Event + ?Sized, S: Initialized<Ev>> Sourced<Initial<Ev>>
    for Option<S>
{
    fn apply(&mut self, event: &Initial<Ev>) {
        *self = Some(S::init(&event.0));
    }
}
