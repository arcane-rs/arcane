use std::convert::Infallible;

use arcana::es::adapter::{
    self,
    transformer::{strategy, Strategy},
};

use crate::event;

impl adapter::WithError for Adapter {
    type Error = Infallible;
    type Transformed = event::Chat;
}

#[derive(Debug, Strategy)]
#[strategy(
    strategy::Initialized => (
        event::chat::public::Created,
        event::chat::private::Created,
    ),
    strategy::AsIs => event::message::Posted,
    strategy::Skip => (
        event::email::v1::AddedAndConfirmed,
        event::email::Confirmed,
        event::email::Added,
    ),
    strategy::Initialized<strategy::Into<event::chat::private::Created>> => (
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
