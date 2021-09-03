pub mod chat;
pub mod email;
pub mod message;

use arcana::es::{event, Event};
use derive_more::From;

#[derive(Debug, Event, From)]
pub enum Chat {
    PrivateCreated(event::Initial<chat::private::Created>),
    PublicCreated(event::Initial<chat::public::Created>),
    MessagePosted(message::Posted),
}

#[derive(Debug, Event, From)]
pub enum Email {
    Added(event::Initial<email::Added>),
    Confirmed(email::Confirmed),
}

#[derive(Debug, Event, From)]
pub enum Message {
    MessagePosted(event::Initial<message::Posted>),
}

#[cfg(test)]
mod spec {
    use super::{chat, message, Chat, Event as _, Message};

    #[test]
    fn event_names() {
        let ev = Chat::PrivateCreated(chat::private::Created.into());
        assert_eq!(ev.name(), "chat.private.created");

        let ev = Chat::PublicCreated(chat::public::Created.into());
        assert_eq!(ev.name(), "chat.public.created");

        let ev = Chat::MessagePosted(message::Posted);
        assert_eq!(ev.name(), "message.posted");

        let ev = Message::MessagePosted(message::Posted.into());
        assert_eq!(ev.name(), "message.posted");
    }
}
