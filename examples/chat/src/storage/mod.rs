pub mod chat;
pub mod message;

use std::{any::Any, convert::Infallible};

use arcana::es::{
    self,
    adapter::{transformer::strategy, Transformer},
};
use derive_more::From;

use crate::event;

#[derive(Debug, es::Event, From, Transformer)]
#[event(
    transformer(
        adapter = chat::Adapter,
        into = event::Chat,
        ctx = dyn Any,
        err = Infallible,
    ),
    transformer(
        adapter = message::Adapter,
        into = event::Message,
        ctx = dyn Any,
        err = Infallible,
    ),
)]
pub enum Event {
    Chat(ChatEvent),
    Message(MessageEvent),
}

#[derive(Debug, es::Event, From, Transformer)]
#[event(
    transformer(
        adapter = chat::Adapter,
        into = event::Chat,
        ctx = dyn Any,
        err = Infallible,
    ),
)]
pub enum ChatEvent {
    Created(event::chat::v1::Created),
    PublicCreated(event::chat::public::Created),
    PrivateCreated(event::chat::private::Created),
}

#[derive(Debug, es::Event, From, Transformer)]
#[event(
    transformer(
        adapter = chat::Adapter,
        into = event::Chat,
        ctx = dyn Any,
        err = Infallible,
    ),
    transformer(
        adapter = message::Adapter,
        into = event::Message,
        ctx = dyn Any,
        err = Infallible,
    ),
)]
pub enum MessageEvent {
    Posted(event::message::Posted),
}

impl From<strategy::Unknown> for event::Message {
    fn from(u: strategy::Unknown) -> Self {
        match u {}
    }
}
