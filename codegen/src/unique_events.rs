//! Utils for ensuring that every [`Event`] variant has a unique combination of
//! [`Event::name()`] and [`Event::ver()`].
//!
//! # Explanation
//!
//! Main idea is that every [`Event`] or [`VersionedEvent`] deriver generates
//! `const fn __arcana_events() -> [(&'static str, u16); size]` method. This
//! array consists of [`EventName`]s and [`EventVersion`]s of all
//! [`VersionedEvent`]s. Uniqueness is checked with [`const_assert`] of
//! [`has_duplicates`].
//!
//!
//! [`const_assert`]: static_assertions::const_assert
//! [`Event`]: arcana_core::Event
//! [`Event::name()`]: arcana_core::Event::name()
//! [`Event::ver()`]: arcana_core::Event::ver()
//! [`EventName`]: arcana_core::EventName
//! [`EventVersion`]: arcana_core::EventVersion
//! [`VersionedEvent`]: arcana_core::VersionedEvent

/// Trait for keeping track of number of [`VersionedEvent`]s.
///
/// [`VersionedEvent`]: arcana_core::VersionedEvent
pub trait UniqueEvents {
    /// Number of [`VersionedEvent`]s in this [`Event`].
    ///
    /// [`Event`]: arcana_core::Event
    /// [`VersionedEvent`]: arcana_core::VersionedEvent
    const COUNT: usize;
}

/// Checks if array has duplicates.
#[must_use]
pub const fn has_duplicates<const N: usize>(events: [(&str, u16); N]) -> bool {
    let mut outer = 0;
    while outer < events.len() {
        let mut inner = outer + 1;
        while inner < events.len() {
            let (inner_name, inner_ver) = events[inner];
            let (outer_name, outer_ver) = events[outer];
            if str_eq(inner_name, outer_name) && inner_ver == outer_ver {
                return true;
            }
            inner += 1;
        }
        outer += 1;
    }

    false
}

/// Compares strings constantly.
///
/// As there is no `const impl Trait` and `l == r` calls [`Eq`], we have to
/// write custom comparison function.
///
/// [`Eq`]: std::cmp::Eq
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
