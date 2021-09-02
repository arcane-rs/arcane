#![feature(generic_associated_types)]

mod event;
mod storage;

use std::array;

use arcana::es::EventAdapter as _;
use futures::{stream, Stream, TryStreamExt as _};

#[allow(clippy::semicolon_if_nothing_returned)]
#[tokio::main]
async fn main() {
    let events = storage::chat::Adapter
        .transform_all(incoming_events(), &())
        .try_collect::<Vec<event::Chat>>()
        .await
        .unwrap();
    println!("{:?}", events);
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
