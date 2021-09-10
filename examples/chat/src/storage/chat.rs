use std::{any::Any, convert::Infallible};

use arcana::es::adapter::{
    transformer::{self, strategy},
    Transformer,
};

use crate::event;

#[derive(Debug, Transformer)]
#[event(
    transformer(
        from(
            super::Event,
            super::ChatEvent,
            super::MessageEvent,
            super::EmailEvent,
        ),
        into = event::Chat,
        context = dyn Any,
        error = Infallible,
    ),
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
