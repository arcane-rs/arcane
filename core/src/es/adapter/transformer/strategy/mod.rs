//! [`Strategy`] definition and default implementations.

pub mod as_is;
pub mod custom;
pub mod initialized;
pub mod into;
pub mod skip;
pub mod split;

use futures::Stream;

use crate::es::{adapter, event};

use super::{Transformer, WithStrategy};

#[doc(inline)]
pub use self::{
    as_is::AsIs,
    custom::{Custom, Customize},
    initialized::Initialized,
    into::Into,
    skip::Skip,
    split::{Split, Splitter},
};

/// Generalized [`Transformer`] for [`Versioned`] events.
///
/// [`Versioned`]: event::Versioned
pub trait Strategy<Adapter, Event, Ctx>
where
    Event: event::Versioned,
    Ctx: ?Sized,
{
    /// Error of this [`Strategy`].
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
                <Self as Strategy<Adapter, Event, Ctx>>::Transformed,
                <Self as Strategy<Adapter, Event, Ctx>>::Error,
            >,
        > + 'out;

    /// Converts incoming [`Event`] into [`Transformed`].
    ///
    /// [`Event`]: crate::es::Event
    /// [`Transformed`]: Self::Transformed
    fn transform<'me, 'ctx, 'out>(
        adapter: &'me Adapter,
        event: Event,
        context: &'ctx Ctx,
    ) -> Self::TransformedStream<'out>
    where
        'me: 'out,
        'ctx: 'out;
}

impl<Event, Adapter, Ctx> Transformer<Event, Ctx> for adapter::Wrapper<Adapter>
where
    Ctx: ?Sized,
    Event: event::Versioned,
    Adapter: WithStrategy<Event> + adapter::Returning,
    Adapter::Strategy: Strategy<Adapter, Event, Ctx>,
    <Adapter as adapter::Returning>::Transformed:
        From<<Adapter::Strategy as Strategy<Adapter, Event, Ctx>>::Transformed>,
    <Adapter as adapter::Returning>::Error:
        From<<Adapter::Strategy as Strategy<Adapter, Event, Ctx>>::Error>,
{
    type Error = <Adapter::Strategy as Strategy<Adapter, Event, Ctx>>::Error;
    type Transformed =
        <Adapter::Strategy as Strategy<Adapter, Event, Ctx>>::Transformed;
    type TransformedStream<'out> = <Adapter::Strategy as Strategy<
        Adapter,
        Event,
        Ctx,
    >>::TransformedStream<'out>;

    fn transform<'me, 'ctx, 'out>(
        &'me self,
        event: Event,
        context: &'ctx Ctx,
    ) -> Self::TransformedStream<'out>
    where
        'me: 'out,
        'ctx: 'out,
    {
        <Adapter::Strategy as Strategy<Adapter, Event, Ctx>>::transform(
            &self.0, event, context,
        )
    }
}
