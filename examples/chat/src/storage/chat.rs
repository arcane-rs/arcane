use arcana::es::adapter::transformer::{self, strategy};

use crate::event;

#[derive(Debug)]
pub struct Adapter;

impl transformer::WithStrategy<event::chat::public::Created> for Adapter {
    type Strategy = strategy::Initialized<strategy::AsIs>;
}

impl transformer::WithStrategy<event::chat::private::Created> for Adapter {
    type Strategy = strategy::Initialized<strategy::AsIs>;
}

impl transformer::WithStrategy<event::message::Posted> for Adapter {
    type Strategy = strategy::AsIs;
}

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
