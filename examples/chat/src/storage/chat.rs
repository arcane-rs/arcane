use std::convert::Infallible;

use arcana::es::adapter::{
    self,
    transformer::{self, strategy},
};

use crate::event;

impl adapter::WithError for Adapter {
    type Error = Infallible;
    type Transformed = event::Chat;
}

#[derive(Clone, Copy, Debug)]
pub struct Adapter;

impl<Ctx> transformer::WithStrategy<event::chat::public::Created, Ctx>
    for Adapter
{
    type Strategy = strategy::Initialized;
}

impl<Ctx> transformer::WithStrategy<event::chat::private::Created, Ctx>
    for Adapter
{
    type Strategy = strategy::Initialized;
}

impl<Ctx> transformer::WithStrategy<event::chat::v1::Created, Ctx> for Adapter {
    type Strategy =
        strategy::Initialized<strategy::Into<event::chat::private::Created>>;
}

impl<Ctx> transformer::WithStrategy<event::message::Posted, Ctx> for Adapter {
    type Strategy = strategy::AsIs;
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

// Chats are private by default.
impl From<event::chat::v1::Created> for event::chat::private::Created {
    fn from(_: event::chat::v1::Created) -> Self {
        Self
    }
}
