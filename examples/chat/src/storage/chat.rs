use std::{any::Any, convert::Infallible};

use arcana::es::adapter::{
    self,
    transformer::{self, strategy},
    Transformer,
};

use crate::event;

impl adapter::WithError for Adapter {
    type Context = dyn Any;
    type Error = Infallible;
    type Transformed = event::Chat;
}

#[derive(Debug, Transformer)]
#[transformer(
    strategy::Initialized => (
        event::chat::public::Created,
        event::chat::private::Created,
    ),
    strategy::AsIs => event::message::Posted,
    strategy::Skip => (
        event::email::v1::AddedAndConfirmed,
        event::email::Confirmed,
        event::email::Added,
    )
)]
pub struct Adapter;

impl transformer::WithStrategy<event::chat::v1::Created> for Adapter {
    type Strategy =
        strategy::Initialized<strategy::Into<event::chat::private::Created>>;
}

// Chats are private by default.
impl From<event::chat::v1::Created> for event::chat::private::Created {
    fn from(_: event::chat::v1::Created) -> Self {
        Self
    }
}
