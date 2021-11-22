use arcana::es::{
    self,
    event::adapter::{self, strategy, Adapt, Strategy},
};
use futures::stream;

use crate::event;

#[derive(es::EventAdapter, Clone, Copy, Debug)]
#[adapter(into = event::Chat)]
pub struct Adapter;

impl Adapt<event::chat::public::Created> for Adapter {
    type Strategy = strategy::AsIs;
}

impl Adapt<event::chat::private::Created> for Adapter {
    type Strategy = strategy::AsIs;
}

impl Adapt<event::message::Posted> for Adapter {
    type Strategy = strategy::AsIs;
}

impl Adapt<event::chat::v1::Created> for Adapter {
    type Strategy = strategy::Into<event::chat::private::Created>;
}

impl Adapt<event::email::v2::AddedAndConfirmed> for Adapter {
    type Strategy = CustomSkip;
}

impl Adapt<event::email::Confirmed> for Adapter {
    type Strategy = CustomSkip;
}

impl Adapt<event::email::Added> for Adapter {
    type Strategy = CustomSkip;
}

impl Adapt<event::Raw<event::email::v2::AddedAndConfirmed, serde_json::Value>>
    for Adapter
{
    type Strategy = CustomSkip;
}

// Chats are private by default.
impl From<event::chat::v1::Created> for event::chat::private::Created {
    fn from(_: event::chat::v1::Created) -> Self {
        Self
    }
}

/// Custom [`strategy::Skip`] implementation.
pub struct CustomSkip;

impl<Adapter, Event> Strategy<Adapter, Event> for CustomSkip
where
    Adapter: adapter::Returning,
    Adapter::Transformed: 'static,
    Adapter::Error: 'static,
{
    type Context = ();
    type Error = Adapter::Error;
    type Transformed = Adapter::Transformed;
    type TransformedStream<'o>
    where
        Adapter: 'o,
    = stream::Empty<Result<Self::Transformed, Self::Error>>;

    fn transform<'me: 'out, 'ctx: 'out, 'out>(
        _: &'me Adapter,
        _: Event,
        _: &'ctx Self::Context,
    ) -> Self::TransformedStream<'out> {
        stream::empty()
    }
}
