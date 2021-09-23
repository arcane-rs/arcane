use std::convert::Infallible;

use arcana::es::adapter::{
    self,
    transformer::strategy::{Initialized, Skip},
    Transformer,
};

use crate::event;

#[derive(Debug, Transformer)]
#[transformer(
    Initialized => (
        event::message::Posted,
    ),
    Skip => (
        event::chat::public::Created,
        event::chat::private::Created,
        event::chat::v1::Created,
        event::email::Added,
        event::email::Confirmed,
        event::email::v1::AddedAndConfirmed,
    ),
)]
pub struct Adapter;

impl adapter::Returning for Adapter {
    type Error = Infallible;
    type Transformed = event::Message;
}
