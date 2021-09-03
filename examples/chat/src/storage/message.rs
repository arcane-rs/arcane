use arcana::es::adapter::transformer::{self, strategy};

use crate::event;

pub struct Adapter;

impl transformer::WithStrategy<super::ChatEvent> for Adapter {
    type Strategy = strategy::Skip;
}

impl transformer::WithStrategy<event::message::Posted> for Adapter {
    type Strategy = strategy::Initialized<strategy::AsIs>;
}

impl transformer::WithStrategy<super::EmailEvent> for Adapter {
    type Strategy = strategy::Skip;
}
