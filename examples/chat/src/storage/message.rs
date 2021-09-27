use std::convert::Infallible;

use arcana::es::adapter::{
    self,
    transformer::{strategy, Strategy},
};
use futures::stream;

use crate::event;

impl adapter::WithError for Adapter {
    type Error = Infallible;
    type Transformed = event::Message;
}

#[derive(Debug, Strategy)]
#[strategy(
    strategy::Initialized => (
        event::message::Posted,
    ),
    strategy::Skip => (
        event::chat::private::Created,
        event::chat::v1::Created,
        event::email::Added,
        event::email::Confirmed,
        event::email::v1::AddedAndConfirmed,
    ),
    strategy::Custom => event::chat::public::Created,
)]
pub struct Adapter;

// Basically same as Skip, but with additional Ctx bounds
impl<Ctx> strategy::CustomTransformer<event::chat::public::Created, Ctx>
    for Adapter
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
