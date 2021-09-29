//! [`Custom`] [`Strategy`] definition.

use futures::Stream;

use crate::es::event;

use super::Strategy;

/// [`Strategy`] for some custom conversion provided by [`Customize`].
#[derive(Clone, Copy, Debug)]
pub struct Custom;

/// Convert `Event` into [`Stream`] of [`Transformed`] for [`Custom`]
/// [`Strategy`].
///
/// [`Transformed`]: Self::Transformed
pub trait Customize<Event: event::VersionedOrRaw> {
    type Context: ?Sized;

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
                <Self as Customize<Event>>::Transformed,
                <Self as Customize<Event>>::Error,
            >,
        > + 'out;

    /// Converts incoming [`Event`] into [`Transformed`].
    ///
    /// [`Event`]: crate::es::Event
    /// [`Transformed`]: Self::Transformed
    fn transform<'me: 'out, 'ctx: 'out, 'out>(
        &'me self,
        event: Event,
        context: &'ctx Self::Context,
    ) -> Self::TransformedStream<'out>;
}

impl<Adapter, Event> Strategy<Adapter, Event> for Custom
where
    Adapter: Customize<Event>,
    Event: event::VersionedOrRaw,
{
    type Context = <Adapter as Customize<Event>>::Context;
    type Error = Adapter::Error;
    type Transformed = Adapter::Transformed;
    type TransformedStream<'out> = Adapter::TransformedStream<'out>;

    fn transform<'me: 'out, 'ctx: 'out, 'out>(
        adapter: &'me Adapter,
        event: Event,
        context: &'ctx Self::Context,
    ) -> Self::TransformedStream<'out> {
        Adapter::transform(adapter, event, context)
    }
}
