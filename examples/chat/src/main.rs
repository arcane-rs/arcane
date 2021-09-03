#![feature(generic_associated_types)]

mod domain;
mod event;
mod storage;

use std::array;

use arcana::es::{event::Sourced, EventAdapter as _};
use futures::{stream, Stream, TryStreamExt as _};

#[allow(clippy::semicolon_if_nothing_returned)]
#[tokio::main]
async fn main() {
    let mut chat = Option::<domain::Chat>::None;
    let mut message = Option::<domain::Message>::None;

    let chat_events = storage::chat::Adapter
        .transform_all(incoming_events(), &())
        .inspect_ok(|ev| chat.apply(ev))
        .try_collect::<Vec<event::Chat>>()
        .await
        .unwrap();
    println!("{:?}", chat_events);

    assert_eq!(
        chat,
        Some(domain::Chat {
            visibility: domain::chat::Visibility::Public,
            message_count: 1
        }),
    );

    let message_events = storage::message::Adapter
        .transform_all(incoming_events(), &())
        .inspect_ok(|ev| message.apply(ev))
        .try_collect::<Vec<event::Message>>()
        .await
        .unwrap();
    println!("{:?}", message_events);

    assert_eq!(message, Some(domain::Message));
}

fn incoming_events() -> impl Stream<Item = storage::Event> {
    stream::iter(array::IntoIter::new([
        storage::ChatEvent::Created(event::chat::v1::Created).into(),
        storage::ChatEvent::PrivateCreated(event::chat::private::Created)
            .into(),
        storage::ChatEvent::PublicCreated(event::chat::public::Created).into(),
        storage::MessageEvent::Posted(event::message::Posted).into(),
    ]))
}
