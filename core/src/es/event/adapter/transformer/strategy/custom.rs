//! [`Custom`] [`Strategy`] definition.

use futures::Stream;

use super::{event, Strategy};

/// [`Strategy`] for some custom conversion provided by [`Customize`].
///
/// This [`Strategy`] should be used in case you don't plan to reuse
/// [`Customize`] impl. Otherwise you should implement [`Strategy`] on your
/// custom struct and reuse it.
#[derive(Clone, Copy, Debug)]
pub struct Custom;

/// Convert `Event` into [`Stream`] of [`Transformed`] for [`Custom`]
/// [`Strategy`].
///
/// [`Transformed`]: Self::Transformed
pub trait Customize<Event: event::VersionedOrRaw> {
    /// Context of this [`Custom`] [`Strategy`].
    ///
    /// This should be one of 2 things:
    /// - `()`
    ///   In case you want to accept any struct as a `context`.
    /// - [`spell::Borrowed`]`<dyn Trait>`
    ///   In that case `context` should be able to be [`Borrow`]ed as
    ///   `dyn Trait`.
    ///
    /// [`Borrow`]: std::borrow::Borrow
    /// [`spell::Borrowed`]: crate::spell::Borrowed
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
        > + 'out
    where
        Self: 'out;

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
    type TransformedStream<'out>
    where
        Adapter: 'out,
    = Adapter::TransformedStream<'out>;

    fn transform<'me: 'out, 'ctx: 'out, 'out>(
        adapter: &'me Adapter,
        event: Event,
        context: &'ctx Self::Context,
    ) -> Self::TransformedStream<'out> {
        Adapter::transform(adapter, event, context)
    }
}
