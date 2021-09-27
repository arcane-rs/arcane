use std::convert::Infallible;

use arcana::es::adapter::{self, strategy, WithStrategy};

use crate::event;

impl adapter::Returning for Adapter {
    type Error = Infallible;
    type Transformed = event::Chat;
}

#[derive(Clone, Copy, Debug)]
pub struct Adapter;

impl WithStrategy<event::chat::public::Created> for Adapter {
    type Strategy = strategy::Initialized;
}

impl WithStrategy<event::chat::private::Created> for Adapter {
    type Strategy = strategy::Initialized;
}

impl WithStrategy<event::chat::v1::Created> for Adapter {
    type Strategy =
        strategy::Initialized<strategy::Into<event::chat::private::Created>>;
}

impl WithStrategy<event::message::Posted> for Adapter {
    type Strategy = strategy::AsIs;
}

impl WithStrategy<event::email::v1::AddedAndConfirmed> for Adapter {
    type Strategy = strategy::Skip;
}

impl WithStrategy<event::email::Confirmed> for Adapter {
    type Strategy = strategy::Skip;
}

impl WithStrategy<event::email::Added> for Adapter {
    type Strategy = strategy::Skip;
}

// Chats are private by default.
impl From<event::chat::v1::Created> for event::chat::private::Created {
    fn from(_: event::chat::v1::Created) -> Self {
        Self
    }
}
