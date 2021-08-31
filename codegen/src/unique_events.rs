//! Utils for ensuring in compile time that every [`Event`] variant's
//! combination of [`Event::name`] and [`Event::version`] is corresponding to
//! single Rust type.
//!
//! # Explanation
//!
//! Main idea is that every [`Event`] or [`event::Versioned`] deriving generates
//! a hidden
//! `const fn __arcana_events() -> [(&'static str, &'static str, u16); size]`
//! method.
//! This array consists of unique Rust type identifiers, [`event::Name`]s and
//! [`event::Version`]s of all the [`Event`] variants. Correctness is checked
//! then with [`const_assert`]ing the [`has_duplicates()`] function.
//!
//! [`const_assert`]: static_assertions::const_assert
//! [`Event`]: arcana_core::es::Event
//! [`Event::name`]: arcana_core::es::Event::name
//! [`Event::version`]: arcana_core::es::Event::version
//! [`event::Name`]: arcana_core::es::event::Name
//! [`event::Version`]: arcana_core::es::event::Version
//! [`event::Versioned`]: arcana_core::es::event::Versioned

/// Tracking of number of [`VersionedEvent`]s.
///
/// [`VersionedEvent`]: arcana_core::es::VersionedEvent
pub trait UniqueEvents {
    /// Number of [`VersionedEvent`]s in this [`Event`].
    ///
    /// [`Event`]: arcana_core::es::Event
    /// [`VersionedEvent`]: arcana_core::es::VersionedEvent
    const COUNT: usize;
}

/// Checks whether the given array of `events` combinations of [`Event::name`]
/// and [`Event::version`] corresponding to different Rust types.
///
/// [`Event::name`]: arcana_core::es::Event::name
/// [`Event::version`]: arcana_core::es::Event::version
#[must_use]
pub const fn has_duplicates<const N: usize>(
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
#[must_use]
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
