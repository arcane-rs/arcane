//! [`Event`] machinery.

use std::{convert::TryFrom, num::NonZeroU16};

use derive_more::{Deref, DerefMut, Display, Into};
use ref_cast::RefCast;
use sealed::sealed;

/// Fully qualified name of an [`Event`].
pub type Name = &'static str;

/// Revision number of an [`Event`].
#[derive(
    Clone, Copy, Debug, Display, Eq, Hash, Into, Ord, PartialEq, PartialOrd,
)]
// TODO: Should it be bigger? Allow to abstract over it?
pub struct Version(NonZeroU16);

impl Version {
    /// Creates a new [`Version`] out of the given `value`.
    ///
    /// The given `value` should not be `0` (zero) and fit into [`u16`] size.
    #[must_use]
    pub fn try_new<N>(value: N) -> Option<Self>
    where
        u16: TryFrom<N>,
    {
        Some(Self(NonZeroU16::new(u16::try_from(value).ok()?)?))
    }

    /// Creates a new [`Version`] out of the given `value` without checking its
    /// invariants.
    ///
    /// # Safety
    ///
    /// The given `value` must not be `0` (zero).
    #[inline]
    #[must_use]
    pub const unsafe fn new_unchecked(value: u16) -> Self {
        Self(NonZeroU16::new_unchecked(value))
    }
}

/// [`Event`] of a concrete [`Version`].
pub trait Versioned {
    /// Returns [`Name`] of this [`Event`].
    ///
    /// _Note:_ This should effectively be a constant value, and should never
    /// change.
    #[must_use]
    fn name() -> Name;

    /// Returns [`Version`] of this [`Event`].
    ///
    /// _Note:_ This should effectively be a constant value, and should never
    /// change.
    #[must_use]
    fn version() -> Version;
}

/// [Event Sourcing] event describing something that has occurred (happened
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
    fn version(&self) -> Version;
}

impl<Ev: Versioned + ?Sized> Event for Ev {
    fn name(&self) -> Name {
        <Self as Versioned>::name()
    }

    fn version(&self) -> Version {
        <Self as Versioned>::version()
    }
}

/// State that can be calculated by applying the specified [`Event`].
pub trait Sourced<Ev: ?Sized> {
    /// Applies the specified [`Event`] to the current state.
    fn apply(&mut self, event: &Ev);
}

impl<Ev: Event + ?Sized, S: Sourced<Ev>> Sourced<Ev> for Option<S> {
    fn apply(&mut self, event: &Ev) {
        if let Some(state) = self {
            state.apply(event);
        }
    }
}

/// Before a state can be [`Sourced`] it needs to be [`Initialized`].
pub trait Initialized<Ev: ?Sized> {
    /// Creates an initial state from the given [`Event`].
    fn init(event: &Ev) -> Self;
}

/// Wrapper type to mark an [`Event`] that makes some [`Sourced`] state being
/// [`Initialized`].
#[derive(Clone, Copy, Debug, Deref, DerefMut, Display, RefCast)]
#[repr(transparent)]
pub struct Initial<Ev: ?Sized>(pub Ev);

impl<Ev> From<Ev> for Initial<Ev> {
    fn from(ev: Ev) -> Self {
        Initial(ev)
    }
}

impl<Ev: Event + ?Sized, S: Initialized<Ev>> Sourced<Initial<Ev>>
    for Option<S>
{
    fn apply(&mut self, event: &Initial<Ev>) {
        *self = Some(S::init(&event.0));
    }
}

/// [`Borrow`]-like trait for borrowing [`Event`]s as is or from [`Initial`].
/// Used in codegen only.
#[sealed]
pub trait BorrowInitial<Borrowed: ?Sized> {
    /// Borrows [`Event`].
    fn borrow(&self) -> &Borrowed;
}

#[sealed]
impl<Ev: Event + ?Sized> BorrowInitial<Ev> for Initial<Ev> {
    fn borrow(&self) -> &Ev {
        &self.0
    }
}

#[sealed]
impl<T: Event + ?Sized> BorrowInitial<T> for T {
    fn borrow(&self) -> &T {
        self
    }
}

/// Trait for getting [`Event`] as is or from [`Initial`]. Used in codegen only.
#[sealed]
pub trait UnpackInitial {
    /// [`Event`] type.
    type Event: ?Sized;
}

#[sealed]
impl<Ev: Event + ?Sized> UnpackInitial for Initial<Ev> {
    type Event = Ev;
}

#[sealed]
impl<Ev: Event + ?Sized> UnpackInitial for Ev {
    type Event = Ev;
}
