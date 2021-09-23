//! [`Transformer`] definitions.

pub mod strategy;

use futures::Stream;

use crate::es::{adapter::Correct, event};

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
    /// TODO
    type Context<Impl>: Correct;

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
    type TransformedStream<'out, Ctx: 'out>: Stream<
            Item = Result<
                <Self as Transformer<Event>>::Transformed,
                <Self as Transformer<Event>>::Error,
            >,
        > + 'out;

    /// Converts incoming [`Event`] into [`Transformed`].
    ///
    /// [`Event`]: crate::es::Event
    /// [`Transformed`]: Self::Transformed
    fn transform<'me, 'ctx, 'out, Ctx>(
        &'me self,
        event: Event,
        context: &'ctx Ctx,
    ) -> Self::TransformedStream<'out, Ctx>
    where
        'me: 'out,
        'ctx: 'out,
        Ctx: 'out;
}

/// Instead of implementing [`Transformer`] manually, you can use this trait
/// with some [`Strategy`].
pub trait WithStrategy<Event>
where
    Self: Sized,
    Event: event::Versioned,
{
    /// [`Strategy`] to transform [`Event`] with.
    ///
    /// [`Event`]: crate::es::Event
    type Strategy: Strategy<Self, Event>;
}
