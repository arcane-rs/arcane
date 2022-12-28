//! [`Event`] machinery.

use std::num::NonZeroU16;

use derive_more::{Deref, DerefMut, Display, Into};
use ref_cast::RefCast;

/// Fully qualified name of an [`Event`].
pub type Name = &'static str;

/// Revision number of an [`Event`].
#[derive(
    Clone, Copy, Debug, Display, Eq, Hash, Into, Ord, PartialEq, PartialOrd,
)]
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

/// [`Event`] of a concrete [`Version`].
pub trait Versioned {
    /// [`Name`] of this [`Event`].
    const NAME: Name;

    /// [`Version`] of this [`Event`].
    const VERSION: Version;
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
        <Self as Versioned>::NAME
    }

    fn version(&self) -> Version {
        <Self as Versioned>::VERSION
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

impl<'e, S: Sourced<dyn Event + Send + 'e>> Sourced<dyn Event + Send + 'e>
    for Option<S>
{
    fn apply(&mut self, event: &(dyn Event + Send + 'e)) {
        if let Some(state) = self {
            state.apply(event);
        }
    }
}

impl<'e, S: Sourced<dyn Event + Send + Sync + 'e>>
    Sourced<dyn Event + Send + Sync + 'e> for Option<S>
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
/// # use arcane::es::event::{self, Sourced as _};
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

impl<Ev: Event + ?Sized, S: Initialized<Ev>> Sourced<Initial<Ev>>
    for Option<S>
{
    fn apply(&mut self, event: &Initial<Ev>) {
        *self = Some(S::init(&event.0));
    }
}

/// Trait for reflecting over [`Event`]'s [`Name`]s and [`Version`]s.
pub trait VersionedNames {
    /// Iterator over all this [`Event`]'s versioned names.
    type Iterator: Iterator<Item = (Name, Version)>;

    /// Returns iterator over this [`Event`]'s versioned names.
    fn versioned_names() -> Self::Iterator;
}

#[cfg(feature = "codegen")]
pub mod codegen {
    //! [`Event`] machinery aiding codegen.
    //!
    //! [`Event`]: super::Event

    use core::slice;

    use super::{Name, Version, VersionedNames};

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

    /// Trait contains all combinations of [`Name`]s and [`Version`]s of this
    /// [`Event`], corresponding to their Rust types.
    ///
    /// # Safety
    ///
    /// This trait is generated by `#[derive(event::Versioned)]` and not part of
    /// the public API. Do not use it directly. Instead, use [`VersionedNames`].
    ///
    /// [`Event`]: super::Event
    pub unsafe trait Events {
        /// All combinations of unique Rust type identifiers with their
        /// [`Name`]s and [`Version`]s.
        const EVENTS: &'static [(&'static str, &'static str, u16)];
    }

    /// Iterator over all combinations of [`Name`]s and [`Version`]s of the
    /// [`Event`].
    ///
    /// **Note**: This iterator is not meant to be used directly. Instead, use
    ///           [`VersionedNames::Iterator`].
    ///
    /// [`Event`]: super::Event
    #[derive(Clone, Debug)]
    pub struct VersionedNamesIterator(
        slice::Iter<'static, (&'static str, &'static str, u16)>,
    );

    impl VersionedNamesIterator {
        /// Creates a new [`VersionedNamesIterator`] from [`Events::EVENTS`].
        fn new<E: Events>() -> Self {
            Self(<E as Events>::EVENTS.iter())
        }
    }

    impl Iterator for VersionedNamesIterator {
        type Item = (Name, Version);

        fn next(&mut self) -> Option<Self::Item> {
            self.0.next().map(|(_, name, ver)| {
                // SAFETY: Safe, because `VersionedNamesIterator` can be
                //         constructed only inside of this module and only from
                //         `Events::EVENTS` which is guaranteed to contain valid
                //         `Version`s (and requires `unsafe` to implement it).
                let ver = unsafe { Version::new_unchecked(*ver) };
                (*name, ver)
            })
        }
    }

    impl<T: Events> VersionedNames for T {
        type Iterator = VersionedNamesIterator;

        fn versioned_names() -> Self::Iterator {
            // TODO: Use `has_different_types_with_same_name_and_ver` as const
            //       here, once rust-lang/rust#57775 is resolved:
            //       https://github.com/rust-lang/rust/issues/57775

            VersionedNamesIterator::new::<T>()
        }
    }

    /// Checks in compile time whether all the given combinations of
    /// [`Event::name`] and [`Event::version`] correspond to different Rust
    /// types.
    ///
    /// # Explanation
    ///
    /// Main idea is that every [`Event`] or [`event::Versioned`] deriving
    /// generates an [`Events`] implementation:
    /// ```rust,ignore
    /// impl Events for MyEvent {
    ///     const EVENTS: &'static [(&'static str, &'static str, u16)] = &[
    ///         ("unique_type_id1", "my.event.created", 1),
    ///         ("unique_type_id2", "my.event.deleted", 1)
    ///     ];
    /// }
    /// ```
    /// It returns a slice consisting of unique Rust type identifiers,
    /// [`event::Name`]s and [`event::Version`]s of all the [`Event`] variants.
    /// Correctness is checked then with asserting this function at compile time
    /// in `const` context.
    ///
    /// [`Event`]: super::Event
    /// [`Event::name`]: super::Event::name
    /// [`Event::version`]: super::Event::version
    /// [`event::Name`]: super::Name
    /// [`event::Version`]: super::Version
    /// [`event::Versioned`]: super::Versioned
    #[must_use]
    pub const fn has_different_types_with_same_name_and_ver<E: Events>() -> bool
    {
        let events = <E as Events>::EVENTS;

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
}
