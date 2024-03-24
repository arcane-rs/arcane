//! [`Event`] machinery.

use std::{borrow::Cow, num::NonZeroU16};

use derive_more::{Deref, DerefMut, Display, Into};
use ref_cast::RefCast;
use sealed::sealed;

/// Fully qualified name of an [`Event`].
pub type Name = &'static str;

/// Abstracted [`Revision`] number of an [`Event`].
pub trait Revision: Copy + ToString {}

impl Revision for &str {}

impl Revision for Version {}

/// [`NonZeroU16`] incremental [`Revision`] number of an [`Event`].
#[derive(
    Clone, Copy, Debug, Display, Eq, Hash, Into, Ord, PartialEq, PartialOrd,
)]
pub struct Version(NonZeroU16);

impl Version {
    /// Creates a new [`Version`] out of the provided `value`.
    ///
    /// The provided `value` should not be `0` (zero) and fit into [`u16`] size.
    #[must_use]
    pub fn try_new<N>(value: N) -> Option<Self>
    where
        u16: TryFrom<N>,
    {
        Some(Self(NonZeroU16::new(u16::try_from(value).ok()?)?))
    }

    /// Creates a new [`Version`] out of the provided `value` without checking
    /// its invariants.
    ///
    /// # Safety
    ///
    /// The provided `value` must not be `0` (zero).
    #[inline]
    #[must_use]
    pub const unsafe fn new_unchecked(value: u16) -> Self {
        // SAFETY: Safety invariants are the same as for this method.
        Self(unsafe { NonZeroU16::new_unchecked(value) })
    }

    /// Returns the value of this [`Version`] as a primitive type.
    #[inline]
    #[must_use]
    pub const fn get(self) -> u16 {
        self.0.get()
    }
}

/// [Event Sourcing] event describing something that has occurred (happened
/// fact).
///
/// [Event Sourcing]: https://martinfowler.com/eaaDev/EventSourcing.html
pub trait Event {
    /// Returns [`Name`] of this [`Event`].
    ///
    /// > **NOTE:** This should effectively be a constant value, and should
    /// >           never change.
    #[must_use]
    fn name(&self) -> Name;
}

/// Concrete [`Event`] defined statically.
pub trait Static: Event {
    /// Concrete [`Name`] of this [`Event`].
    const NAME: Name;
}

impl<Ev: Static + ?Sized> Event for Ev {
    fn name(&self) -> Name {
        <Self as Static>::NAME
    }
}

/// [`Event`] capable of evolving with time.
pub trait Revisable: Event {
    /// Type of this [`Event`]'s [`Revision`] number.
    type Revision: Revision;

    /// Returns [`Revision`] of this [`Event`].
    #[must_use]
    fn revision(&self) -> Self::Revision;
}

/// Shortcut for naming a [`Revision`] of a [`RevisableEvent`].
///
/// [`RevisableEvent`]: Revisable
pub type RevisionOf<Ev> = <Ev as Revisable>::Revision;

/// [`StaticEvent`] of a concrete [`Revision`].
///
/// [`StaticEvent`]: Static
pub trait Concrete: Revisable + Static {
    /// Type of this [`StaticEvent`]'s [`Revision`] number.
    ///
    /// [`StaticEvent`]: Static
    type Revision: Revision;

    /// Concrete [`Revision`] of this [`Event`].
    const REVISION: RevisionOf<Self>;
}

impl<Ev: Concrete + ?Sized> Revisable for Ev {
    type Revision = <Self as Concrete>::Revision;

    fn revision(&self) -> Self::Revision {
        <Self as Concrete>::REVISION
    }
}

/// State that can be calculated by applying the specified [`Event`].
pub trait Sourced<Ev: ?Sized> {
    /// Applies the specified [`Event`] to the current state.
    fn apply(&mut self, event: &Ev);
}

impl<Ev, S> Sourced<Ev> for Option<S>
where
    Ev: Concrete + ?Sized,
    S: Sourced<Ev>,
{
    fn apply(&mut self, event: &Ev) {
        if let Some(state) = self {
            state.apply(event);
        }
    }
}

impl<'e, S> Sourced<dyn Event + 'e> for Option<S>
where
    S: Sourced<dyn Event + 'e>,
{
    fn apply(&mut self, event: &(dyn Event + 'e)) {
        if let Some(state) = self {
            state.apply(event);
        }
    }
}

impl<'e, S> Sourced<dyn Event + Send + 'e> for Option<S>
where
    S: Sourced<dyn Event + Send + 'e>,
{
    fn apply(&mut self, event: &(dyn Event + Send + 'e)) {
        if let Some(state) = self {
            state.apply(event);
        }
    }
}

impl<'e, S> Sourced<dyn Event + Send + Sync + 'e> for Option<S>
where
    S: Sourced<dyn Event + Send + Sync + 'e>,
{
    fn apply(&mut self, event: &(dyn Event + Send + Sync + 'e)) {
        if let Some(state) = self {
            state.apply(event);
        }
    }
}

/// [`Event`] sourcing the specified state.
///
/// This is a reversed version of [`Sourced`] trait intended to simplify usage
/// of trait objects describing sets of [`Event`]s. Shouldn't be implemented
/// manually, but rather used as blanket impl.
///
/// # Example
///
/// ```rust
/// # use arcane::es::event::{self, Event, Sourced as _};
/// #
/// #[derive(Debug, Eq, PartialEq)]
/// struct Chat;
///
/// #[derive(Event)]
/// #[event(name = "chat", revision = 1)]
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
#[sealed]
pub trait Sourcing<S: ?Sized> {
    /// Applies this [`Event`] to the specified `state`.
    fn apply_to(&self, state: &mut S);
}

#[sealed]
impl<Ev: ?Sized, S: Sourced<Ev> + ?Sized> Sourcing<S> for Ev {
    fn apply_to(&self, state: &mut S) {
        state.apply(self);
    }
}

impl<'e, S: ?Sized> Sourced<dyn Sourcing<S> + 'e> for S {
    fn apply(&mut self, event: &(dyn Sourcing<S> + 'e)) {
        event.apply_to(self);
    }
}

impl<'e, S: ?Sized> Sourced<dyn Sourcing<S> + Send + 'e> for S {
    fn apply(&mut self, event: &(dyn Sourcing<S> + Send + 'e)) {
        event.apply_to(self);
    }
}

impl<'e, S: ?Sized> Sourced<dyn Sourcing<S> + Send + Sync + 'e> for S {
    fn apply(&mut self, event: &(dyn Sourcing<S> + Send + Sync + 'e)) {
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
///
/// Exists solely to solve specialization problems.
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

impl<Ev, S> Sourced<Initial<Ev>> for Option<S>
where
    Ev: Event + ?Sized,
    S: Initialized<Ev>,
{
    fn apply(&mut self, event: &Initial<Ev>) {
        *self = Some(S::init(&event.0));
    }
}

/// Raw [`Event`] representation.
#[derive(Clone, Debug)]
pub struct Raw<'name, Data, Rev = ()> {
    /// [`Name`] of the [`Event`].
    pub name: Cow<'name, str>,

    /// [`Revision`] of the [`Event`].
    pub revision: Rev,

    /// [`Event`]'s data.
    pub data: Data,
}

/// Error of converting [`Raw`] event to [`Event`].
#[derive(Clone, Debug)]
pub enum FromRawError<FromDataError, Rev> {
    /// No [`Event`] associated `name` and `revision` pair found.
    UnknownEvent {
        /// Name of the unknown [`Event`].
        name: String,

        /// [`Revision`] of the [`Event`].
        revision: Rev,
    },

    /// Failed to decode the [`Raw`]'s data.
    FromDataError(FromDataError),
}

#[cfg(feature = "reflect")]
pub mod reflect {
    //! Compile-time reflection for [`Event`] machinery.

    use std::iter;

    use super::super::{event, Event};

    /// Compile-time reflection of a single or multiple [`StaticEvent`]s.
    ///
    /// > **NOTE**: Implementations of this trait are automatically generated by
    /// >           `#[derive(Event)]` macro, and shouldn't be written manually.
    /// >           The trait is exposed for usage purposes only.
    ///
    /// [`StaticEvent`]: event::Static
    pub trait Static: Event {
        /// List of all [`StaticEvent::NAME`]s this [`Event`] is composed of
        /// (including multiple levels of composition).
        ///
        /// > **NOTE**: May contain duplicates if the same [`event::Name`] is
        /// >           used by multiple nested [`StaticEvent`]s.
        ///
        /// [`StaticEvent`]: event::Static
        /// [`StaticEvent::NAME`]: event::Static::NAME
        const NAMES: &'static [event::Name];
    }

    /// Compile-time reflection of a single or multiple [`ConcreteEvent`]s.
    ///
    /// > **NOTE**: Implementations of this trait are automatically generated by
    /// >           `#[derive(Event)]` macro, and shouldn't be written manually.
    /// >           The trait is exposed for usage purposes only.
    ///
    /// [`ConcreteEvent`]: event::Concrete
    pub trait Concrete: event::Revisable + Static
    where
        event::RevisionOf<Self>: 'static,
    {
        /// List of all [`ConcreteEvent::REVISION`]s this [`RevisableEvent`] is
        /// composed of (including multiple levels of composition).
        ///
        /// > **NOTE**: May contain duplicates if the same [`event::Revision`]
        /// >           is used by multiple nested [`ConcreteEvent`]s.
        ///
        /// [`ConcreteEvent`]: event::Concrete
        /// [`ConcreteEvent::REVISION`]: event::Concrete::REVISION
        /// [`RevisableEvent`]: event::Revisable
        const REVISIONS: &'static [event::RevisionOf<Self>];

        /// Returns an [`Iterator`] over all the pairs of [`StaticEvent::NAME`]
        /// and [`ConcreteEvent::REVISION`] this [`RevisableEvent`] is composed
        /// of (including multiple levels of composition).
        ///
        /// > **NOTE**: May contain duplicates if the same [`ConcreteEvent`]
        /// >           is used multiple times in this [`RevisableEvent`].
        ///
        /// [`ConcreteEvent`]: event::Concrete
        /// [`ConcreteEvent::REVISION`]: event::Concrete::REVISION
        /// [`RevisableEvent`]: event::Revisable
        /// [`StaticEvent::NAME`]: event::Static::NAME
        // TODO: Make `const` once `const fn` is allowed in traits.
        fn names_and_revisions_iter() -> impl Iterator<
            Item = (&'static event::Name, &'static event::RevisionOf<Self>),
        > {
            iter::zip(Self::NAMES, Self::REVISIONS)
        }
    }
}
