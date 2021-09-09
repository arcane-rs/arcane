use std::{any::Any, convert::Infallible};

use arcana::es::adapter::Transformer;

use crate::event;

#[derive(Debug, Transformer)]
#[event(
    transformer(
        from(
            (super::Event, number_of_events = 3),
            (super::MessageEvent, number_of_events = 1),
            (super::ChatEvent, number_of_events = 3),
            (super::EmailEvent, number_of_events = 3),
        ),
        into = event::Message,
        context = dyn Any,
        error = Infallible,
    ),
)]
pub struct Adapter;
