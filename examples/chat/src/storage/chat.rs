use std::convert::Infallible;

use arcana::es::adapter::{self, strategy, Adapt};

use crate::event;

impl adapter::Returning for Adapter {
    type Error = Infallible;
    type Transformed = event::Chat;
}

#[derive(Clone, Copy, Debug)]
pub struct Adapter;

impl Adapt<event::chat::public::Created> for Adapter {
    type Strategy = strategy::Initialized;
}

impl Adapt<event::chat::private::Created> for Adapter {
    type Strategy = strategy::Initialized;
}

impl Adapt<event::chat::v1::Created> for Adapter {
    type Strategy =
        strategy::Initialized<strategy::Into<event::chat::private::Created>>;
}

impl Adapt<event::message::Posted> for Adapter {
    type Strategy = strategy::AsIs;
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

// Chats are private by default.
impl From<event::chat::v1::Created> for event::chat::private::Created {
    fn from(_: event::chat::v1::Created) -> Self {
        Self
    }
}
