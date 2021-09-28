use std::convert::Infallible;

use arcana::es::adapter::{self, strategy, WithStrategy};
use futures::stream;

use crate::event;

impl adapter::Returning for Adapter {
    type Error = Infallible;
    type Transformed = event::Message;
}

#[derive(Debug)]
pub struct Adapter;

impl WithStrategy<event::message::Posted> for Adapter {
    type Strategy = strategy::Initialized;
}

impl WithStrategy<event::chat::public::Created> for Adapter {
    type Strategy = strategy::Custom;
}

impl WithStrategy<event::chat::private::Created> for Adapter {
    type Strategy = strategy::Skip;
}

impl WithStrategy<event::chat::v1::Created> for Adapter {
    type Strategy = strategy::Skip;
}

impl WithStrategy<event::email::v2::AddedAndConfirmed> for Adapter {
    type Strategy = strategy::Skip;
}

impl WithStrategy<event::email::Confirmed> for Adapter {
    type Strategy = strategy::Skip;
}

impl WithStrategy<event::email::Added> for Adapter {
    type Strategy = strategy::Skip;
}

impl
    WithStrategy<
        event::Raw<event::email::v2::AddedAndConfirmed, serde_json::Value>,
    > for Adapter
{
    type Strategy = strategy::Skip;
}

// Basically same as Skip, but with additional Ctx bounds
impl<Ctx> strategy::Customize<event::chat::public::Created, Ctx> for Adapter
where
    Ctx: From<i32>,
{
    type Error = Infallible;
    type Transformed = event::Message;
    type TransformedStream<'out> =
        stream::Empty<Result<Self::Transformed, Self::Error>>;

    fn transform<'me, 'ctx, 'out>(
        &'me self,
        _event: event::chat::public::Created,
        _context: &'ctx Ctx,
    ) -> Self::TransformedStream<'out>
    where
        'me: 'out,
        'ctx: 'out,
    {
        stream::empty()
    }
}
