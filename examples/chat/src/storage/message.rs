use std::convert::Infallible;

use arcana::es::adapter::{
    self,
    transformer::{
        self,
        strategy::{Initialized, Skip},
    },
};

use crate::event;

impl adapter::WithError for Adapter {
    type Error = Infallible;
    type Transformed = event::Message;
}

// #[derive(Debug, Transformer)]
// #[transformer(
//     Initialized => (
//         event::message::Posted,
//     ),
//     Skip => (
//         event::chat::public::Created,
//         event::chat::private::Created,
//         event::chat::v1::Created,
//         event::email::Added,
//         event::email::Confirmed,
//         event::email::v1::AddedAndConfirmed,
//     ),
// )]
pub struct Adapter;

impl transformer::WithStrategy<event::message::Posted> for Adapter {
    type Strategy = Initialized;
}

impl transformer::WithStrategy<event::chat::public::Created> for Adapter {
    type Strategy = Skip;
}

impl transformer::WithStrategy<event::chat::private::Created> for Adapter {
    type Strategy = Skip;
}

impl transformer::WithStrategy<event::chat::v1::Created> for Adapter {
    type Strategy = Skip;
}

impl transformer::WithStrategy<event::email::Added> for Adapter {
    type Strategy = Skip;
}

impl transformer::WithStrategy<event::email::Confirmed> for Adapter {
    type Strategy = Skip;
}

impl transformer::WithStrategy<event::email::v1::AddedAndConfirmed>
    for Adapter
{
    type Strategy = Skip;
}
