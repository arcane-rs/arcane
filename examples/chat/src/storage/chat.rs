use std::convert::Infallible;

use arcana::es::adapter::{
    self,
    transformer::{
        self,
        strategy::{AsIs, Initialized, Into, Skip},
    },
};

use crate::event;

impl adapter::WithError for Adapter {
    type Error = Infallible;
    type Transformed = event::Chat;
}

// #[derive(Debug, Transformer)]
// #[transformer(
//     Initialized => (
//         event::chat::public::Created,
//         event::chat::private::Created,
//     ),
//     AsIs => event::message::Posted,
//     Skip => (
//         event::email::v1::AddedAndConfirmed,
//         event::email::Confirmed,
//         event::email::Added,
//     ),
//     Initialized<Into<event::chat::private::Created>> => (
//         event::chat::v1::Created,
//     ),
// )]
#[derive(Debug)]
pub struct Adapter;

impl transformer::WithStrategy<event::chat::public::Created> for Adapter {
    type Strategy = Initialized;
}

impl transformer::WithStrategy<event::chat::private::Created> for Adapter {
    type Strategy = Initialized;
}

impl transformer::WithStrategy<event::message::Posted> for Adapter {
    type Strategy = AsIs;
}

impl transformer::WithStrategy<event::email::v1::AddedAndConfirmed>
    for Adapter
{
    type Strategy = Skip;
}

impl transformer::WithStrategy<event::email::Confirmed> for Adapter {
    type Strategy = Skip;
}

impl transformer::WithStrategy<event::email::Added> for Adapter {
    type Strategy = Skip;
}

impl transformer::WithStrategy<event::chat::v1::Created> for Adapter {
    type Strategy = Initialized<Into<event::chat::private::Created>>;
}

// Chats are private by default.
impl From<event::chat::v1::Created> for event::chat::private::Created {
    fn from(_: event::chat::v1::Created) -> Self {
        Self
    }
}
