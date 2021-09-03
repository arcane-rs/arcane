pub mod chat;
pub mod email;
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
        adapter = email::Adapter,
        into = event::Email,
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
    Email(EmailEvent),
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

#[derive(Debug, es::Event, From, Transformer)]
#[event(
    transformer(
        adapter = email::Adapter,
        into = event::Email,
        ctx = dyn Any,
        err = Infallible,
    ),
)]
pub enum EmailEvent {
    Added(event::email::Added),
    Confirmed(event::email::Confirmed),
    AddedAndConfirmed(event::email::v1::AddedAndConfirmed),
}

impl From<strategy::Unknown> for event::Chat {
    fn from(u: strategy::Unknown) -> Self {
        match u {}
    }
}

impl From<strategy::Unknown> for event::Email {
    fn from(u: strategy::Unknown) -> Self {
        match u {}
    }
}

impl From<strategy::Unknown> for event::Message {
    fn from(u: strategy::Unknown) -> Self {
        match u {}
    }
}
