#![feature(generic_associated_types)]

mod event;
mod storage;

use std::array;

use arcana::es::adapter::Adapter as _;
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

fn incoming_events() -> impl Stream<Item = storage::Events> {
    stream::iter(array::IntoIter::new([
        storage::ChatEvents::Created(event::chat::v1::Created).into(),
        storage::ChatEvents::PrivateCreated(event::chat::private::Created)
            .into(),
        storage::ChatEvents::PublicCreated(event::chat::public::Created).into(),
        storage::MessageEvents::Posted(event::message::Posted).into(),
    ]))
}
