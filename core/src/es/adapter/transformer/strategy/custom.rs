//! [`Custom`] [`Strategy`] definition.

use futures::Stream;

use crate::es::event;

use super::Strategy;

/// [`Strategy`] for some custom conversion provided by [`Customize`].
#[derive(Clone, Copy, Debug)]
pub struct Custom;

/// Convert `Event` into [`Stream`] of [`Transformed`] [`Event`]s for [`Custom`]
/// [`Strategy`].
///
/// [`Event`]: event::Event
/// [`Transformed`]: Self::Transformed
pub trait Customize<Event, Ctx>
where
    Event: event::VersionedOrRaw,
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
                <Self as Customize<Event, Ctx>>::Transformed,
                <Self as Customize<Event, Ctx>>::Error,
            >,
        > + 'out;

    /// Converts incoming [`Event`] into [`Transformed`].
    ///
    /// [`Event`]: crate::es::Event
    /// [`Transformed`]: Self::Transformed
    fn transform<'me, 'ctx, 'out>(
        &'me self,
        event: Event,
        context: &'ctx Ctx,
    ) -> Self::TransformedStream<'out>
    where
        'me: 'out,
        'ctx: 'out;
}

impl<Adapter, Event, Ctx> Strategy<Adapter, Event, Ctx> for Custom
where
    Adapter: Customize<Event, Ctx>,
    Event: event::VersionedOrRaw,
{
    type Error = Adapter::Error;
    type Transformed = Adapter::Transformed;
    type TransformedStream<'out> = Adapter::TransformedStream<'out>;

    fn transform<'me, 'ctx, 'out>(
        adapter: &'me Adapter,
        event: Event,
        context: &'ctx Ctx,
    ) -> Self::TransformedStream<'out>
    where
        'me: 'out,
        'ctx: 'out,
    {
        Adapter::transform(adapter, event, context)
    }
}
