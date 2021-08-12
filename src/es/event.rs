use std::{convert::TryFrom, num::NonZeroU16};

use derive_more::Display;
use safety_guard::safety;

//use crate::cqrs::Aggregate;

#[cfg(feature = "codegen")]
#[doc(inline)]
pub use arcana_codegen::Event;

pub trait Event {
    /// Fully qualified name of this [`Event`].
    #[must_use]
    fn fqn(&self) -> Fqn;

    /// [`Revision`] number of this [`Event`].
    #[must_use]
    fn revision(&self) -> Revision;
}

/// [`Event`] having a concrete unique type.
pub trait Typed {
    /// Fully qualified name of the type of this [`Event`].
    const FQN: Fqn;

    /// [`Revision`] number of the type of this [`Event`].
    const REVISION: Revision;
}

/*
pub trait OfAggregate: Event {
    type Aggregate: Aggregate;

    fn type_names() -> &'static [TypeName];
}

pub trait OfDomain: Event {}

pub trait Sourced<Ev: Event + ?Sized> {
    fn apply(&mut self, event: &Ev);
}

pub trait Initialized<Ev: Event + ?Sized> {
    fn initialize(event: &Ev) -> Self;
}

impl<Agg, Ev> Sourced<Ev> for Agg
where Agg: ?Sized,
     Ev: Event + Sourcing<Agg> + ?Sized
{
    #[inline]
    fn apply(&mut self, event: &Ev) {
        event.apply_to(self)
    }
}

pub trait Sourcing<Agg: ?Sized>: Event {
    fn apply_to(&self, aggregate: &mut Agg);
}*/
/*
#[derive(Debug, Copy, Clone, Hash, PartialEq, Eq)]
pub struct Numbered<Ev> {
    pub num: Number,

    pub data: Ev,
}

#[derive(Clone, Copy, Debug, Display, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Number(NonZeroU128);

impl Number {
    #[allow(unsafe_code)]
    pub const MIN_VALUE: Self = Self(unsafe { NonZeroU128::new_unchecked(1) });

    #[inline]
    pub fn new<N: Into<u128>>(x: N) -> Option<Self> {
        Some(Self(NonZeroU128::new(x.into())?))
    }

    #[inline]
    pub fn incr(&mut self) {
        self.0 = unsafe { NonZeroU128::new_unchecked(self.0.get() + 1) };
    }

    #[inline]
    #[must_use]
    pub fn next(mut self) -> Self {
        self.incr();
        self
    }
}
*/

/// Fully qualified name of an [`Event`].
pub type Fqn = &'static str;

/// Revision number of an [`Event`].
#[derive(
    Clone, Copy, Debug, Display, Eq, Hash, Into, Ord, PartialEq, PartialOrd,
)]
#[into(forward)]
pub struct Revision(NonZeroU16);

impl Revision {
    /// Creates a new [`Revision`] out of the given `val`ue.
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

    /// Creates a new [`Revision`] out of the given `val`ue without checking its
    /// invariants.
    #[inline]
    #[must_use]
    #[safety(ne(n, 0), "The given `val`ue must not be `0` (zero).")]
    pub const unsafe fn new_unchecked(val: u16) -> Self {
        Self(NonZeroU16::new_unchecked(val))
    }
}
