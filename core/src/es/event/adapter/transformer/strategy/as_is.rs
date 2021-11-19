//! [`AsIs`] [`Strategy`] definition.

use futures::{future, stream};

use super::{
    event::{self, adapter},
    Strategy,
};

/// [`Strategy`] for passing [`Event`]s as is, without any conversions.
///
/// [`Event`]: crate::es::Event
#[derive(Clone, Copy, Debug)]
pub struct AsIs;

impl<Adapter, Event> Strategy<Adapter, Event> for AsIs
where
    Adapter: adapter::Returning,
    Adapter::Error: 'static,
    Event: event::VersionedOrRaw + 'static,
{
    type Context = ();
    type Error = Adapter::Error;
    type Transformed = Event;
    #[allow(unused_lifetimes)] // false positive
    type TransformedStream<'o>
    where
        Adapter: 'o,
    = stream::Once<future::Ready<Result<Self::Transformed, Self::Error>>>;

    #[allow(unused_lifetimes)] // false positive
    fn transform<'me: 'out, 'ctx: 'out, 'out>(
        _: &Adapter,
        event: Event,
        _: &Self::Context,
    ) -> Self::TransformedStream<'out> {
        stream::once(future::ready(Ok(event)))
    }
}
