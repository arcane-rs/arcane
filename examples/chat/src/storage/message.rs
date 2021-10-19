use std::{borrow::Borrow, convert::Infallible};

use arcana::es::{
    self,
    event::adapter::{strategy, Adapt},
};
use futures::stream;

use crate::event;

#[derive(es::EventAdapter, Debug)]
#[adapter(into = event::Message)]
pub struct Adapter;

impl Adapt<event::message::Posted> for Adapter {
    type Strategy = strategy::AsIs;
}

impl Adapt<event::chat::public::Created> for Adapter {
    type Strategy = strategy::Custom;
}

impl Adapt<event::chat::private::Created> for Adapter {
    type Strategy = strategy::Skip;
}

impl Adapt<event::chat::v1::Created> for Adapter {
    type Strategy = strategy::Skip;
}

impl Adapt<event::email::v2::AddedAndConfirmed> for Adapter {
    type Strategy = strategy::Skip;
}

impl Adapt<event::email::Confirmed> for Adapter {
    type Strategy = strategy::Skip;
}

impl Adapt<event::email::Added> for Adapter {
    type Strategy = strategy::Skip;
}

impl Adapt<event::Raw<event::email::v2::AddedAndConfirmed, serde_json::Value>>
    for Adapter
{
    type Strategy = strategy::Skip;
}

// Basically same as Skip, but with additional Context bounds.
impl strategy::Customize<event::chat::public::Created> for Adapter {
    type Context = dyn Bound;
    type Error = Infallible;
    type Transformed = event::Message;
    type TransformedStream<'out> =
        stream::Empty<Result<Self::Transformed, Self::Error>>;

    fn transform<'me, 'ctx, 'out>(
        &'me self,
        _event: event::chat::public::Created,
        _context: &'ctx Self::Context,
    ) -> Self::TransformedStream<'out>
    where
        'me: 'out,
        'ctx: 'out,
    {
        stream::empty()
    }
}

pub trait Bound {}

impl Bound for () {}

impl Borrow<(dyn Bound + 'static)> for () {
    fn borrow(&self) -> &(dyn Bound + 'static) {
        self
    }
}
