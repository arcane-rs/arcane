pub mod chat;
pub mod email;
pub mod message;

use arcana::es;
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
    AddedAndConfirmed(event::email::v2::AddedAndConfirmed),
    RawAddedAndConfirmed(
        event::Raw<event::email::v2::AddedAndConfirmed, serde_json::Value>,
    ),
}

#[cfg(test)]
mod spec {
    use std::array;

    use arcana::es::{EventAdapter as _, EventSourced as _};
    use futures::{future, stream, Stream, TryStreamExt as _};
    use serde_json::json;

    use crate::domain;

    use super::{
        chat, email, event, message, ChatEvent, EmailEvent, Event, MessageEvent,
    };

    #[allow(clippy::semicolon_if_nothing_returned)]
    #[tokio::test]
    async fn chat_adapter() {
        let mut chat = Option::<domain::Chat>::None;
        let chat_events = chat::Adapter
            .transform_all(incoming_events(), &())
            .inspect_ok(|ev| chat.apply(ev))
            .try_collect::<Vec<_>>()
            .await
            .unwrap();

        assert_eq!(
            chat_events,
            vec![
                event::chat::private::Created.into(),
                event::chat::private::Created.into(),
                event::chat::public::Created.into(),
                event::message::Posted.into()
            ]
        );
        assert_eq!(
            chat,
            Some(domain::Chat {
                visibility: domain::chat::Visibility::Public,
                message_count: 1
            }),
        );
    }

    #[allow(clippy::semicolon_if_nothing_returned)]
    #[tokio::test]
    async fn email_adapter() {
        let mut email = Option::<domain::Email>::None;
        let email_events = email::Adapter
            .transform_all(incoming_events(), &"test")
            .inspect_ok(|ev| email.apply(ev))
            .try_collect::<Vec<_>>()
            .await
            .unwrap();

        assert_eq!(
            email_events,
            vec![
                event::email::Added {
                    email: "hello@world.com".to_string()
                }
                .into(),
                event::email::Added {
                    email: "raw@event.com".to_string()
                }
                .into(),
                event::email::Confirmed {
                    confirmed_by: "User".to_string()
                }
                .into(),
            ]
        );
        assert_eq!(
            email,
            Some(domain::Email {
                value: "raw@event.com".to_owned(),
                confirmed_by: Some("User".to_owned()),
            })
        );
    }

    #[allow(clippy::semicolon_if_nothing_returned)]
    #[tokio::test]
    async fn email_adapter_with_corrupted_event() {
        let corrupted_event =
            Event::from(EmailEvent::RawAddedAndConfirmed(event::Raw::new(
                json!({ "corrupted": "raw@event.com", "confirmed_by": "User" }),
                event::Version::try_new(1).unwrap(),
            )));

        let result = email::Adapter
            .transform_all(stream::once(future::ready(corrupted_event)), &42)
            .try_collect::<Vec<_>>()
            .await;

        assert_eq!(result.unwrap_err().to_string(), "missing field `email`")
    }

    #[allow(clippy::semicolon_if_nothing_returned)]
    #[tokio::test]
    async fn message_adapter() {
        let mut message = Option::<domain::Message>::None;
        let message_events = message::Adapter
            .transform_all(incoming_events(), &message::EmptyProvider)
            .inspect_ok(|ev| message.apply(ev))
            .try_collect::<Vec<event::Message>>()
            .await
            .unwrap();

        assert_eq!(message_events, vec![event::message::Posted.into()]);
        assert_eq!(message, Some(domain::Message));
    }

    fn incoming_events() -> impl Stream<Item = Event> {
        stream::iter(array::IntoIter::new([
            ChatEvent::Created(event::chat::v1::Created).into(),
            ChatEvent::PrivateCreated(event::chat::private::Created).into(),
            ChatEvent::PublicCreated(event::chat::public::Created).into(),
            MessageEvent::Posted(event::message::Posted).into(),
            EmailEvent::AddedAndConfirmed(
                event::email::v2::AddedAndConfirmed {
                    email: "hello@world.com".to_owned(),
                    confirmed_by: None,
                },
            )
            .into(),
            EmailEvent::RawAddedAndConfirmed(event::Raw::new(
                json!({ "email": "raw@event.com", "confirmed_by": "User" }),
                event::Version::try_new(1).unwrap(),
            ))
            .into(),
        ]))
    }
}
