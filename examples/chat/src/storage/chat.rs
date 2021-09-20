use std::convert::Infallible;

use arcana::es::adapter::{
    self,
    transformer::strategy::{AsIs, Initialized, Into, Skip},
    Transformer,
};

use crate::event;

impl<Ctx> adapter::WithError<Ctx> for Adapter {
    type Error = Infallible;
    type Transformed = event::Chat;
}

#[derive(Debug, Transformer)]
#[transformer(
    Initialized => (
        event::chat::public::Created,
        event::chat::private::Created,
    ),
    AsIs => event::message::Posted,
    Skip => (
        event::email::v1::AddedAndConfirmed,
        event::email::Confirmed,
        event::email::Added,
    ),
    Initialized<Into<event::chat::private::Created>> => (
        event::chat::v1::Created,
    ),
)]
pub struct Adapter;

// Chats are private by default.
impl From<event::chat::v1::Created> for event::chat::private::Created {
    fn from(_: event::chat::v1::Created) -> Self {
        Self
    }
}
