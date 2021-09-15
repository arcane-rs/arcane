use std::{any::Any, convert::Infallible};

use arcana::es::adapter::{self, transformer::strategy, Transformer};

use crate::event;

impl adapter::WithError for Adapter {
    type Context = dyn Any;
    type Error = Infallible;
    type Transformed = event::Message;
}

#[derive(Debug, Transformer)]
#[transformer(
    strategy::Initialized => (
        event::message::Posted,
    ),
    strategy::Skip => (
        event::chat::public::Created,
        event::chat::private::Created,
        event::chat::v1::Created,
        event::email::Added,
        event::email::Confirmed,
        event::email::v1::AddedAndConfirmed,
    ),
)]
pub struct Adapter;
