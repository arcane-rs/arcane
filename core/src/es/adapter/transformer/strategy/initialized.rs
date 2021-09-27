//! [`Initialized`] [`Strategy`] definition.

use std::marker::PhantomData;

use futures::{stream, TryStreamExt as _};

use crate::es::event;

use super::{AsIs, Strategy};

/// [`Strategy`] for wrapping [`Event`]s in [`Initial`].
///
/// [`Event`]: crate::es::Event
/// [`Initial`]: event::Initial
#[derive(Clone, Debug)]
pub struct Initialized<InnerStrategy = AsIs>(PhantomData<InnerStrategy>);

impl<Adapter, Event, InnerStrategy, Ctx> Strategy<Adapter, Event, Ctx>
    for Initialized<InnerStrategy>
where
    Ctx: ?Sized,
    Event: event::Versioned,
    InnerStrategy: Strategy<Adapter, Event, Ctx>,
    InnerStrategy::Transformed: 'static,
    InnerStrategy::Error: 'static,
{
    type Error = InnerStrategy::Error;
    type Transformed = event::Initial<InnerStrategy::Transformed>;
    type TransformedStream<'out> = stream::MapOk<
        InnerStrategy::TransformedStream<'out>,
        WrapInitial<InnerStrategy::Transformed>,
    >;

    fn transform<'me, 'ctx, 'out>(
        adapter: &'me Adapter,
        event: Event,
        context: &'ctx Ctx,
    ) -> Self::TransformedStream<'out>
    where
        'me: 'out,
        'ctx: 'out,
    {
        InnerStrategy::transform(adapter, event, context).map_ok(event::Initial)
    }
}

type WrapInitial<Event> = fn(Event) -> event::Initial<Event>;
