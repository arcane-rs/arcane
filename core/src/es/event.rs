//! [`Event`] machinery.

use std::num::NonZeroU16;

use derive_more::{Deref, DerefMut, Display, Into};
use ref_cast::RefCast;

/// Fully qualified name of an [`Event`].
pub type Name = &'static str;

/// Abstracted [`Revision`] number of an [`Event`].
pub trait Revision: Copy {}

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
pub trait Sourcing<S: ?Sized> {
    /// Applies this [`Event`] to the specified `state`.
    fn apply_to(&self, state: &mut S);
}

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

/// [`Event`] reflection machinery.
pub mod reflect {
    use crate::es::event::{self, Event};

    /// [`Event::name`] reflection.
    ///
    /// **Note**: Implementations of this trait generates by `#[derive(Event)]`
    ///           derive macro, and shouldn't be implemented manually.
    pub trait Name: Event {
        /// List of all [`Event::name`]s.
        ///
        /// Contains the [`Name`]s of all [`Event`]s this [`Event`] is
        /// composed of (including multiple levels of composition).
        ///
        /// Can be [`zip`]ped with [`event::reflect::Revision::REVISIONS`] of
        /// the same [`Event`].
        ///
        /// **Note**: May contains duplicates if the same [`Name`] is used by
        ///           multiple nested [`Event`]s.
        ///
        /// [`Name`]: event::Name
        /// [`zip`]: Iterator::zip
        const NAMES: &'static [event::Name];
    }

    /// [`event::Revisable::revision`] reflection.
    ///
    /// **Note**: Implementations of this trait generates by `#[derive(Event)]`
    ///           derive macro, and shouldn't be implemented manually.
    pub trait Revision: event::Revisable
    where
        <Self as event::Revisable>::Revision: 'static,
    {
        /// List of all [`event::Revisable::revision`]s.
        ///
        /// Contains the [`Revision`]s of all [`Event`]s this [`Event`] is
        /// composed of (including multiple levels of composition).
        ///
        /// Can be [`zip`]ped with [`event::reflect::Name::NAMES`] of the same
        /// [`Event`].
        ///
        /// **Note**: May contains duplicates if the same [`Revision`] is used
        ///           by multiple nested [`Event`]s.
        ///
        /// [`Revision`]: event::Revision
        /// [`zip`]: Iterator::zip
        const REVISIONS: &'static [event::RevisionOf<Self>];
    }
}

#[cfg(feature = "codegen")]
pub mod codegen {
    //! [`Event`] machinery aiding codegen.
    //!
    //! [`Event`]: super::Event

    /// Concatenates slices at compile time.
    #[macro_export]
    macro_rules! const_concat_slices {
        ($ty:ty, $a:expr) => {$a};
        ($ty:ty, $a:expr, $b:expr $(,)*) => {{
            const A: &[$ty] = $a;
            const B: &[$ty] = $b;
            const __LEN: usize = A.len() + B.len();
            const __CONCATENATED: &[$ty; __LEN] = &{
                let mut out: [$ty; __LEN] = if __LEN == 0 {
                    unsafe {
                        ::core::mem::transmute(
                            [0u8; ::core::mem::size_of::<$ty>() * __LEN],
                        )
                    }
                } else if A.len() == 0 {
                    [B[0]; { A.len() + B.len() }]
                } else {
                    [A[0]; { A.len() + B.len() }]
                };
                let mut i = 0;
                while i < A.len() {
                    out[i] = A[i];
                    i += 1;
                }
                i = 0;
                while i < B.len() {
                    out[i + A.len()] = B[i];
                    i += 1;
                }
                out
            };

            __CONCATENATED
        }};
        ($ty:ty, $a:expr, $b:expr, $($c:expr),+ $(,)*) => {
            $crate::const_concat_slices!(
                $ty,
                $a,
                $crate::const_concat_slices!($ty, $b, $($c),+)
            )
        };
    }

    /// Meta information contains all combinations of [`Event::name`]s and
    /// [`event::Revisable::revision`]s of the [`Event`], corresponding to
    /// their Rust types.
    ///
    /// **Note**: Implementations of this trait generates by `#[derive(Event)]`
    ///           derive macro, and not part of the public API.
    ///
    /// [`Event`]: super::Event
    /// [`Event::name`]: super::Event::name
    /// [`event::Revisable::revision`]: super::Revisable::revision
    pub trait Meta {
        /// Meta information of the [`Event`]. Contains:
        /// - Unique Rust type identifier.
        /// - [`Event::name`].
        /// - String representing a [`event::Revisable::revision`].
        ///
        /// [`Event`]: super::Event
        /// [`Event::name`]: super::Event::name
        /// [`event::Revisable::revision`]: super::Revisable::revision
        const META: &'static [(&'static str, &'static str, &'static str)];
    }

    /// Checks whether all the given combinations of [`Event::name`] and
    /// [`event::Revisable::revision`] in [`Meta`] correspond to different Rust
    /// types.
    ///
    /// Correctness is checked by asserting this function at compile time in
    /// `const` context.
    ///
    /// [`Event`]: super::Event
    /// [`Event::name`]: super::Event::name
    /// [`event::Revisable::revision`]: super::Revisable::revision
    #[must_use]
    pub const fn has_different_types_with_same_name_and_revision<E: Meta>(
    ) -> bool {
        let events = <E as Meta>::META;

        let mut outer = 0;
        while outer < events.len() {
            let mut inner = outer + 1;
            while inner < events.len() {
                let (inner_ty, inner_name, inner_rev) = events[inner];
                let (outer_ty, outer_name, outer_rev) = events[outer];

                if !str_eq(inner_ty, outer_ty)
                    && str_eq(inner_name, outer_name)
                    && str_eq(inner_rev, outer_rev)
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
    /// [`Eq`]: trait@Eq
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

    #[cfg(test)]
    mod uniqueness_type_check_spec {
        use super::{has_different_types_with_same_name_and_revision, Meta};

        #[test]
        fn no_when_all_events_are_unique() {
            struct Ev;

            impl Meta for Ev {
                const META: &'static [(
                    &'static str,
                    &'static str,
                    &'static str,
                )] = &[("A", "a", "1"), ("B", "b", "2"), ("C", "c", "3")];
            }

            assert!(!has_different_types_with_same_name_and_revision::<Ev>());
        }

        #[test]
        fn no_when_has_same_types_with_same_name_and_revision() {
            struct Ev;

            impl Meta for Ev {
                const META: &'static [(
                    &'static str,
                    &'static str,
                    &'static str,
                )] = &[("A", "a", "1"), ("A", "a", "1"), ("A", "b", "1")];
            }

            assert!(!has_different_types_with_same_name_and_revision::<Ev>());
        }

        #[test]
        fn no_when_has_same_types_with_same_name_and_empty_revision() {
            struct Ev;

            impl Meta for Ev {
                const META: &'static [(
                    &'static str,
                    &'static str,
                    &'static str,
                )] = &[("A", "a", ""), ("A", "a", ""), ("A", "b", "")];
            }

            assert!(!has_different_types_with_same_name_and_revision::<Ev>());
        }

        #[test]
        fn yes_when_has_different_types_and_same_name_and_revision() {
            struct Ev;

            impl Meta for Ev {
                const META: &'static [(
                    &'static str,
                    &'static str,
                    &'static str,
                )] = &[("A", "a", "1"), ("B", "a", "1"), ("A", "b", "1")];
            }

            assert!(has_different_types_with_same_name_and_revision::<Ev>());
        }

        #[test]
        fn yes_when_one_type_with_empty_revision_and_same_name() {
            struct Ev;

            impl Meta for Ev {
                const META: &'static [(
                    &'static str,
                    &'static str,
                    &'static str,
                )] = &[("A", "a", "1"), ("B", "a", ""), ("A", "b", "1")];
            }

            assert!(!has_different_types_with_same_name_and_revision::<Ev>());
        }

        #[test]
        fn yes_when_has_different_types_with_same_names_without_revisions() {
            struct Ev;

            impl Meta for Ev {
                const META: &'static [(
                    &'static str,
                    &'static str,
                    &'static str,
                )] = &[("A", "a", ""), ("B", "a", ""), ("A", "b", "1")];
            }

            assert!(has_different_types_with_same_name_and_revision::<Ev>());
        }
    }
}
