//! [`Into`] [`Strategy`] definition.

use std::marker::PhantomData;

use futures::{stream, TryStreamExt as _};

use crate::es::event;

use super::{AsIs, Strategy};

/// [`Strategy`] for converting [`Event`]s using [`From`] impl.
///
/// [`Event`]: crate::es::Event
#[derive(Copy, Clone, Debug)]
pub struct Into<I, InnerStrategy = AsIs>(PhantomData<(I, InnerStrategy)>);

impl<Adapter, Event, IntoEvent, InnerStrategy, Ctx>
    Strategy<Adapter, Event, Ctx> for Into<IntoEvent, InnerStrategy>
where
    Ctx: ?Sized,
    Event: event::Versioned,
    InnerStrategy: Strategy<Adapter, Event, Ctx>,
    InnerStrategy::Transformed: 'static,
    InnerStrategy::Error: 'static,
    IntoEvent: From<InnerStrategy::Transformed> + 'static,
{
    type Error = InnerStrategy::Error;
    type Transformed = IntoEvent;
    type TransformedStream<'out> = stream::MapOk<
        InnerStrategy::TransformedStream<'out>,
        IntoFn<InnerStrategy::Transformed, IntoEvent>,
    >;

    fn transform<'me, 'ctx, 'out>(
        adapter: &'me Adapter,
        event: Event,
        ctx: &'ctx Ctx,
    ) -> Self::TransformedStream<'out>
    where
        'me: 'out,
        'ctx: 'out,
    {
        InnerStrategy::transform(adapter, event, ctx).map_ok(IntoEvent::from)
    }
}

type IntoFn<FromEvent, IntoEvent> = fn(FromEvent) -> IntoEvent;
