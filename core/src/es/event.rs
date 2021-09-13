//! [`Event`] machinery.

use std::{convert::TryFrom, num::NonZeroU16};

use derive_more::{Deref, DerefMut, Display, Into};
use ref_cast::RefCast;

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

impl<Ev: Versioned + ?Sized, S: Sourced<Ev>> Sourced<Ev> for Option<S> {
    fn apply(&mut self, event: &Ev) {
        if let Some(state) = self {
            state.apply(event);
        }
    }
}

impl<'e, S: Sourced<dyn Event + 'e>> Sourced<dyn Event + 'e> for Option<S> {
    fn apply(&mut self, event: &(dyn Event + 'e)) {
        if let Some(state) = self {
            state.apply(event);
        }
    }
}

/// [`Event`] that can source state `S`. Shouldn't be implemented manually,
/// rather used as blanket impl.
///
/// # Example
///
/// ```rust
/// # use arcana::es::event::{self, Sourced as _};
/// #
/// #[derive(Debug, Eq, PartialEq)]
/// struct Chat;
///
/// #[derive(event::Versioned)]
/// #[event(name = "chat", version = 1)]
/// struct ChatEvent;
///
/// impl event::Initialized<ChatEvent> for Chat {
///     fn init(_: &ChatEvent) -> Self {
///         Self
///     }
/// }
///
/// let mut chat = Option::<Chat>::None;
/// let ev = event::Initial(ChatEvent);
/// let ev: &dyn event::Sourcing<Option<Chat>> = &ev;
/// chat.apply(ev);
/// assert_eq!(chat, Some(Chat));
/// ```
pub trait Sourcing<S: ?Sized> {
    /// Applies the specified [`Event`] to the current state.
    fn apply_to(&self, state: &mut S);
}

impl<Ev, S: ?Sized> Sourcing<S> for Ev
where
    S: Sourced<Ev>,
{
    fn apply_to(&self, state: &mut S) {
        state.apply(self);
    }
}

impl<'e, S> Sourced<dyn Sourcing<S> + 'e> for S {
    fn apply(&mut self, event: &(dyn Sourcing<S> + 'e)) {
        event.apply_to(self);
    }
}

/// Before a state can be [`Sourced`] it needs to be [`Initialized`].
pub trait Initialized<Ev: ?Sized> {
    /// Creates an initial state from the given [`Event`].
    #[must_use]
    fn init(event: &Ev) -> Self;
}

/// Wrapper type to mark an [`Event`] that makes some [`Sourced`] state being
/// [`Initialized`].
#[derive(Clone, Copy, Debug, Deref, DerefMut, Display, RefCast)]
#[repr(transparent)]
pub struct Initial<Ev: ?Sized>(pub Ev);

// Manual implementation due to `derive_more::From` not being able to strip
// `?Sized` trait bound.
impl<Ev> From<Ev> for Initial<Ev> {
    fn from(ev: Ev) -> Self {
        Self(ev)
    }
}

/// TODO
pub trait TransparentFrom<F> {
    /// TODO
    fn from(_: F) -> Self;
}

impl<L, R> TransparentFrom<Initial<L>> for Initial<R>
where
    R: From<L>,
{
    fn from(ev: Initial<L>) -> Self {
        Initial(ev.0.into())
    }
}

impl<Ev: Event + ?Sized, S: Initialized<Ev>> Sourced<Initial<Ev>>
    for Option<S>
{
    fn apply(&mut self, event: &Initial<Ev>) {
        *self = Some(S::init(&event.0));
    }
}

/// TODO
pub trait Upcast: Sized {
    /// TODO
    type Into: From<Self>;
}

#[cfg(feature = "codegen")]
pub mod codegen {
    //! [`Event`] machinery aiding codegen.

    use sealed::sealed;

    use super::{Event, Initial};

    /// Custom [`Borrow`] codegen aiding trait for borrowing an [`Event`] either
    /// from itself or from an [`Initial`] wrapper.
    ///
    /// [`Borrow`]: std::borrow::Borrow
    #[sealed]
    pub trait Borrow {
        /// Type of a borrowed [`Event`].
        type Event: ?Sized;

        /// Borrows an [`Event`].
        fn borrow(&self) -> &Self::Event;
    }

    #[sealed]
    impl<T: Event + ?Sized> Borrow for T {
        type Event = T;

        fn borrow(&self) -> &Self::Event {
            self
        }
    }

    #[sealed]
    impl<Ev: Event + ?Sized> Borrow for Initial<Ev> {
        type Event = Ev;

        fn borrow(&self) -> &Self::Event {
            &self.0
        }
    }

    /// Codegen aiding trait for retrieving a type of an [`Event`] either from
    /// itself or from an [`Initial`] wrapper.
    #[sealed]
    pub trait Unpacked {
        /// Type of [`Event`] to be retrieved.
        type Type: ?Sized;
    }

    #[sealed]
    impl<Ev: Event + ?Sized> Unpacked for Ev {
        type Type = Ev;
    }

    #[sealed]
    impl<Ev: Event + ?Sized> Unpacked for Initial<Ev> {
        type Type = Ev;
    }

    /// Tracking of [`VersionedEvent`]s number.
    ///
    /// [`VersionedEvent`]: super::Versioned
    pub trait Versioned {
        /// Number of [`VersionedEvent`]s in this [`Event`].
        ///
        /// [`VersionedEvent`]: super::Versioned
        const COUNT: usize;
    }

    impl<Ev: Versioned> Versioned for Initial<Ev> {
        const COUNT: usize = Ev::COUNT;
    }

    /// Checks in compile time whether all the given combinations of
    /// [`Event::name`] and [`Event::version`] correspond to different Rust
    /// types.
    ///
    /// # Explanation
    ///
    /// Main idea is that every [`Event`] or [`event::Versioned`] deriving
    /// generates a hidden method:
    /// ```rust,ignore
    /// const fn __arcana_events() -> [(&'static str, &'static str, u16); size]
    /// ```
    /// It returns an array consisting of unique Rust type identifiers,
    /// [`event::Name`]s and [`event::Version`]s of all the [`Event`] variants.
    /// Correctness is checked then with asserting this function at compile time
    /// in `const` context.
    ///
    /// [`event::Name`]: super::Name
    /// [`event::Version`]: super::Version
    /// [`event::Versioned`]: super::Versioned
    #[must_use]
    pub const fn has_different_types_with_same_name_and_ver<const N: usize>(
        events: [(&str, &str, u16); N],
    ) -> bool {
        let mut outer = 0;
        while outer < events.len() {
            let mut inner = outer + 1;
            while inner < events.len() {
                let (inner_ty, inner_name, inner_ver) = events[inner];
                let (outer_ty, outer_name, outer_ver) = events[outer];
                if !str_eq(inner_ty, outer_ty)
                    && str_eq(inner_name, outer_name)
                    && inner_ver == outer_ver
                {
                    return true;
                }
                inner += 1;
            }
            outer += 1;
        }

        false
    }

    /// Compares strings in `const` context.
    ///
    /// As there is no `const impl Trait` and `l == r` calls [`Eq`], we have to
    /// write custom comparison function.
    ///
    /// [`Eq`]: std::cmp::Eq
    // TODO: Remove once `Eq` trait is allowed in `const` context.
    const fn str_eq(l: &str, r: &str) -> bool {
        if l.len() != r.len() {
            return false;
        }

        let (l, r) = (l.as_bytes(), r.as_bytes());
        let mut i = 0;
        while i < l.len() {
            if l[i] != r[i] {
                return false;
            }
            i += 1;
        }

        true
    }
}
