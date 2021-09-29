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

impl<Adapter, Event, IntoEvent, InnerStrategy> Strategy<Adapter, Event>
    for Into<IntoEvent, InnerStrategy>
where
    Event: event::VersionedOrRaw,
    InnerStrategy: Strategy<Adapter, Event>,
    InnerStrategy::Transformed: 'static,
    InnerStrategy::Error: 'static,
    IntoEvent: From<InnerStrategy::Transformed> + 'static,
{
    type Context = InnerStrategy::Context;
    type Error = InnerStrategy::Error;
    type Transformed = IntoEvent;
    type TransformedStream<'out> = stream::MapOk<
        InnerStrategy::TransformedStream<'out>,
        IntoFn<InnerStrategy::Transformed, IntoEvent>,
    >;

    fn transform<'me: 'out, 'ctx: 'out, 'out>(
        adapter: &'me Adapter,
        event: Event,
        ctx: &'ctx Self::Context,
    ) -> Self::TransformedStream<'out> {
        InnerStrategy::transform(adapter, event, ctx).map_ok(IntoEvent::from)
    }
}

type IntoFn<FromEvent, IntoEvent> = fn(FromEvent) -> IntoEvent;
