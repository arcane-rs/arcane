use std::{any::Any, convert::Infallible};

use arcana::es::adapter::Transformer;

use crate::event;

#[derive(Debug, Transformer)]
#[event(
    transformer(
        from(
            super::Event,
            super::MessageEvent,
            super::ChatEvent,
            super::EmailEvent,
        ),
        into = event::Message,
        context = dyn Any,
        error = Infallible,
    ),
)]
pub struct Adapter;
