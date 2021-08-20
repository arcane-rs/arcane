//! Utils for ensuring that every [`Event`] variant has a unique combination of
//! [`Event::name()`] and [`Event::ver()`].
//!
//! # Explanation
//!
//! Main idea is that every [`Event`] or [`VersionedEvent`] deriver generates
//! `const fn __arcana_events() -> [Option<(&'static str, u16)>; size]`
//! method. Size of outputted array determines max count of unique
//! [`VersionedEvent`]s inside [`Event`] and is tweakable inside
//! `arcana_codegen_impl` crate (default is `100_000` which should be plenty).
//! As these arrays are used only at compile-time, there should be no
//! performance impact at runtime.
//!
//! - Structs
//!
//!   [`unique_event_name_and_ver_for_struct`] macro generates function, which
//!   returns array with only first occupied entry. The rest of them are
//!   [`None`].
//!
//! - Enums
//!
//!   [`unique_event_name_and_ver_for_enum`] macro generates function, which
//!   glues subtypes arrays into single continues array. First `n` entries are
//!   occupied, while the rest of them are [`None`], where `n` is the number of
//!   [`VersionedEvent`]s. As structs deriving [`VersionedEvent`] and enums
//!   deriving [`Event`] have the same output by `__arcana_events()` method,
//!   top-level enum variants can have different levels of nesting.
//!
//!   [`unique_event_name_and_ver_check`] macro generates [`const_assert`]
//!   check, which fails in case of duplicated [`Event::name()`] and
//!   [`Event::ver()`].
//!
//!
//! [`const_assert`]: static_assertions::const_assert
//! [`Event`]: arcana_core::Event
//! [`Event::name()`]: arcana_core::Event::name()
//! [`Event::ver()`]: arcana_core::Event::ver()
//! [`VersionedEvent`]: arcana_core::VersionedEvent

/// Trait for keeping track of number of [`VersionedEvent`]s.
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
