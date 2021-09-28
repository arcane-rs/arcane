//! [`AsIs`] [`Strategy`] definition.

use futures::{future, stream};

use crate::es::{adapter, event};

use super::Strategy;

/// [`Strategy`] for passing [`Event`]s as is, without any conversions.
///
/// [`Event`]: crate::es::Event
#[derive(Clone, Copy, Debug)]
pub struct AsIs;

impl<Adapter, Event, Ctx> Strategy<Adapter, Event, Ctx> for AsIs
where
    Adapter: adapter::Returning,
    Adapter::Error: 'static,
    Ctx: ?Sized,
    Event: event::VersionedOrRaw + 'static,
{
    type Error = Adapter::Error;
    type Transformed = Event;
    type TransformedStream<'out> =
        stream::Once<future::Ready<Result<Self::Transformed, Self::Error>>>;

    fn transform<'me, 'ctx, 'out>(
        _: &'me Adapter,
        event: Event,
        _: &'ctx Ctx,
    ) -> Self::TransformedStream<'out>
    where
        'me: 'out,
        'ctx: 'out,
    {
        stream::once(future::ready(Ok(event)))
    }
}
