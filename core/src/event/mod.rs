//! Event related definitions.

use ref_cast::RefCast;

/// [Event Sourcing] event that describes something that has occurred (happened
/// fact).
///
/// A sequence of [`Event`]s may represent a concrete versioned state of an
/// Aggregate.
///
/// [Event Sourcing]: https://martinfowler.com/eaaDev/EventSourcing.html
pub trait Event {
    /// Returns type of this [`Event`].
    ///
    /// _Note:_ This should effectively be a constant value, and should never
    /// change.
    fn event_type(&self) -> &'static str;

    /// Returns version of this [`Event`].
    fn ver(&self) -> u16;
}

/// Versioned [`Event`].
///
/// The single type of [`Event`] may have different versions, which allows
/// evolving [`Event`] in the type. To overcome the necessity of dealing with
/// multiple types of the same [`Event`], it's recommended for the last actual
/// version of [`Event`] to implement trait [`From`] its previous versions, so
/// they can be automatically transformed into the latest actual version of
pub trait Versioned {
    /// Returns type of this [`Event`].
    ///
    /// _Note:_ This should effectively be a constant value, and should never
    /// change.
    fn event_type() -> &'static str;

    /// Returns version of this [`Event`].
    fn ver() -> u16;
}

impl<Ev: Versioned> Event for Ev {
    fn event_type(&self) -> &'static str {
        <Self as Versioned>::event_type()
    }

    fn ver(&self) -> u16 {
        <Self as Versioned>::ver()
    }
}

/// State that can be calculated by applying specified [`Event`].
pub trait Sourced<Ev: ?Sized> {
    /// Applies given [`Event`] to the current state.
    fn apply(&mut self, event: &Ev);
}

impl<Ev: Event + ?Sized, Agg: Sourced<Ev>> Sourced<Ev> for Option<Agg> {
    fn apply(&mut self, event: &Ev) {
        if let Some(agg) = self {
            agg.apply(event);
        }
    }
}

/// Before items can be [`Sourced`], they need to be [`Initialized`].
pub trait Initialized<Ev: ?Sized> {
    /// Creates initial state from given [`Event`].
    fn init(event: &Ev) -> Self;
}

/// Wrapper-type intended for [`Event`]s that can initialize [`Sourced`] items.
#[derive(Debug, RefCast)]
#[repr(transparent)]
pub struct Initial<Ev: ?Sized>(pub Ev);

impl<Ev: Event + ?Sized, Agg: Initialized<Ev>> Sourced<Initial<Ev>> for Option<Agg> {
    fn apply(&mut self, event: &Initial<Ev>) {
        *self = Some(Agg::init(&event.0));
    }
}
