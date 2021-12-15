//! [`Transformer`] definitions.

pub mod strategy;

use futures::Stream;

#[doc(inline)]
pub use strategy::Strategy;

/// To use [`Adapter`] with some [`Event`], you should provide [`Strategy`]
/// for every [`VersionedEvent`] involved with this [`Event`] and use
/// [`Adapter`] derive macro on struct itself.
///
/// [`Adapter`]: crate::es::event::Adapter
/// [`Event`]: crate::es::Event
/// [`Returning`]: super::Returning
/// [`VersionedEvent`]: crate::es::VersionedEvent
pub trait Adapt<Event> {
    /// [`Strategy`] to transform [`Event`] with.
    ///
    /// [`Event`]: crate::es::Event
    type Strategy;
}

/// Facility to convert [`Event`]s.
/// Typical use cases include (but are not limited to):
///
/// - [`Skip`]ping unused [`Event`]s;
/// - Transforming (ex: from one [`Version`] to another);
/// - [`Split`]ting existing [`Event`]s into more granular ones.
///
/// Provided with blanket impl for [`Adapt`] implementors, so usually you
/// shouldn't implement it manually. For more flexibility consider using
/// [`Custom`] or implementing your own [`Strategy`] in case it will be reused.
/// See [`Adapter`] for more info.
///
/// [`Adapter`]: crate::es::event::Adapter
/// [`Custom`]: strategy::Custom
/// [`Event`]: crate::es::Event
/// [`Skip`]: strategy::Skip
/// [`Split`]: strategy::Split
/// [`Version`]: crate::es::event::Version
pub trait Transformer<'ctx, Event, Ctx: ?Sized> {
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
    type TransformedStream<'out>: Stream<
            Item = Result<
                <Self as Transformer<'ctx, Event, Ctx>>::Transformed,
                <Self as Transformer<'ctx, Event, Ctx>>::Error,
            >,
        > + 'out
    where
        'ctx: 'out,
        Ctx: 'ctx + 'out,
        Self: 'out;

    /// Converts incoming [`Event`] into [`Transformed`].
    ///
    /// [`Event`]: crate::es::Event
    /// [`Transformed`]: Self::Transformed
    fn transform<'me, 'out>(
        &'me self,
        event: Event,
        context: &'ctx Ctx,
    ) -> Self::TransformedStream<'out>
    where
        'me: 'out,
        'ctx: 'out;
}
