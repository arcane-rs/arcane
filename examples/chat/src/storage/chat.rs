use std::{any::Any, convert::Infallible};

use arcana::es::adapter::Transformer;

use crate::event;

#[derive(Debug, Transformer)]
#[event(
    transformer(
        from(
            super::Event,
            super::ChatEvent,
            super::MessageEvent,
            super::EmailEvent,
        ),
        into = event::Chat,
        context = dyn Any,
        error = Infallible,
    ),
)]
pub struct Adapter;
