use arcana::es::{
    adapter::transformer::{self, strategy},
    event::Sourcing,
};

use crate::{domain, event};

#[derive(Debug)]
pub struct Adapter;

impl transformer::WithStrategy<event::chat::public::Created> for Adapter {
    type Strategy = strategy::Initialized<strategy::AsIs>;
}

impl transformer::WithStrategy<event::chat::private::Created> for Adapter {
    type Strategy = strategy::Initialized<strategy::AsIs>;
}

impl transformer::WithStrategy<event::chat::v1::Created> for Adapter {
    type Strategy =
        strategy::Initialized<strategy::Into<event::chat::private::Created>>;
}

impl transformer::WithStrategy<super::EmailEvent> for Adapter {
    type Strategy = strategy::Skip;
}

impl<Ev> transformer::WithStrategy<Ev> for Adapter
where
    Ev: Sourcing<domain::Chat>,
{
    type Strategy = strategy::AsIs;
}

// Chats are private by default.
impl From<event::chat::v1::Created> for event::chat::private::Created {
    fn from(_: event::chat::v1::Created) -> Self {
        Self
    }
}
