//! [`Transformer`] definitions.

pub mod strategy;

use futures::Stream;

#[doc(inline)]
pub use strategy::Strategy;

/// Facility to convert [`Event`]s.
/// Typical use cases include (but are not limited to):
///
/// - [`Skip`]ping unused [`Event`]s;
/// - Transforming (ex: from one [`Version`] to another);
/// - [`Split`]ting existing [`Event`]s into more granular ones.
///
/// To reduce boilerplate consider using [`WithStrategy`] with some [`Strategy`]
/// instead of implementing this trait manually.
///
/// [`Event`]: crate::es::Event
/// [`Skip`]: strategy::Skip
/// [`Split`]: strategy::Split
/// [`Version`]: crate::es::event::Version
pub trait Transformer<Event> {
    /// Context for converting [`Event`]s.
    ///
    /// [`Event`]: crate::es::Event
    type Context: ?Sized;

    /// Error of this [`Transformer`].
    type Error;

    /// Converted [`Event`].
    ///
    /// [`Event`]: crate::es::Event
    type Transformed;

    /// [`Stream`] of [`Transformed`] [`Event`]s.
    ///
    /// [`Event`]: crate::es::Event
    /// [`Transformed`]: Self::Transformed
    type TransformedStream<'me, 'ctx>: Stream<
        Item = Result<Self::Transformed, Self::Error>,
    >;

    /// Converts incoming [`Event`] into [`Transformed`].
    ///
    /// [`Event`]: crate::es::Event
    /// [`Transformed`]: Self::Transformed
    fn transform<'me, 'ctx>(
        &'me self,
        event: Event,
        context: &'ctx Self::Context,
    ) -> Self::TransformedStream<'me, 'ctx>;
}

/// Instead of implementing [`Transformer`] manually, you can use this trait
/// with some [`Strategy`].
pub trait WithStrategy<Event> {
    /// [`Strategy`] to transform [`Event`] with.
    ///
    /// [`Event`]: crate::es::Event
    type Strategy;
}
