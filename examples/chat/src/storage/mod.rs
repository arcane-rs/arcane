pub mod chat;

use std::{any::Any, convert::Infallible};

use arcana::es::adapter::Transformer;
use derive_more::From;

use crate::event;

#[derive(Debug, From, Transformer)]
#[event(
    transformer(
        adapter = chat::Adapter,
        into = event::Chat,
        ctx = dyn Any,
        err = Infallible,
    ),
)]
pub enum Event {
    Chat(ChatEvent),
    Message(MessageEvent),
}

#[derive(Debug, From, Transformer)]
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

#[derive(Debug, From, Transformer)]
#[event(
    transformer(
        adapter = chat::Adapter,
        into = event::Chat,
        ctx = dyn Any,
        err = Infallible,
    ),
)]
pub enum MessageEvent {
    Posted(event::message::Posted),
}
