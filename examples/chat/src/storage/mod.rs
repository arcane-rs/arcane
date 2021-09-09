pub mod chat;
pub mod email;
pub mod message;

use arcana::es::{self, adapter::transformer::strategy};
use derive_more::From;

use crate::event;

#[derive(Debug, es::Event, From)]
pub enum Event {
    Chat(ChatEvent),
    Message(MessageEvent),
    Email(EmailEvent),
}

#[derive(Debug, es::Event, From)]
pub enum ChatEvent {
    Created(event::chat::v1::Created),
    PublicCreated(event::chat::public::Created),
    PrivateCreated(event::chat::private::Created),
}

#[derive(Debug, es::Event, From)]
pub enum MessageEvent {
    Posted(event::message::Posted),
}

#[derive(Debug, es::Event, From)]
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
