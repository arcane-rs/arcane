use std::convert::Infallible;

use arcana::es::adapter::{
    self,
    transformer::{self, strategy},
};
use futures::stream;

use crate::event;

impl adapter::WithError for Adapter {
    type Error = Infallible;
    type Transformed = event::Message;
}

#[derive(Debug)]
pub struct Adapter;

impl<Ctx> transformer::WithStrategy<event::message::Posted, Ctx> for Adapter {
    type Strategy = strategy::Initialized;
}

impl<Ctx> transformer::WithStrategy<event::chat::public::Created, Ctx>
    for Adapter
{
    type Strategy = strategy::Custom;
}

impl<Ctx> transformer::WithStrategy<event::chat::private::Created, Ctx>
    for Adapter
{
    type Strategy = strategy::Skip;
}

impl<Ctx> transformer::WithStrategy<event::chat::v1::Created, Ctx> for Adapter {
    type Strategy = strategy::Skip;
}

impl<Ctx> transformer::WithStrategy<event::email::v1::AddedAndConfirmed, Ctx>
    for Adapter
{
    type Strategy = strategy::Skip;
}

impl<Ctx> transformer::WithStrategy<event::email::Confirmed, Ctx> for Adapter {
    type Strategy = strategy::Skip;
}

impl<Ctx> transformer::WithStrategy<event::email::Added, Ctx> for Adapter {
    type Strategy = strategy::Skip;
}

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
