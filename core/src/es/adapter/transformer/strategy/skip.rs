//! [`Skip`] [`Strategy`] definition.

use futures::stream;

use crate::es::{adapter, event};

use super::Strategy;

/// [`Strategy`] for skipping [`Event`]s.
///
/// [`Event`]: crate::es::Event
#[derive(Clone, Copy, Debug)]
pub struct Skip;

impl<Adapter, Event, Ctx> Strategy<Adapter, Event, Ctx> for Skip
where
    Ctx: ?Sized,
    Event: event::VersionedOrRaw,
    Adapter: adapter::Returning,
    Adapter::Transformed: 'static,
    Adapter::Error: 'static,
{
    type Error = Adapter::Error;
    type Transformed = Adapter::Transformed;
    type TransformedStream<'out> =
        stream::Empty<Result<Self::Transformed, Self::Error>>;

    fn transform<'me, 'ctx, 'out>(
        _: &'me Adapter,
        _: Event,
        _: &'ctx Ctx,
    ) -> Self::TransformedStream<'out>
    where
        'me: 'out,
        'ctx: 'out,
    {
        stream::empty()
    }
}
