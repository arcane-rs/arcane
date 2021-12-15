use arcana::es::event::{Initialized, Sourced};

use crate::event;

#[derive(Debug, Eq, PartialEq)]
pub struct Chat {
    pub visibility: Visibility,
    pub message_count: usize,
}

#[derive(Debug, Eq, PartialEq)]
pub enum Visibility {
    Private,
    Public,
}

impl Initialized<event::chat::public::Created> for Chat {
    fn init(_: &event::chat::public::Created) -> Self {
        Self {
            visibility: Visibility::Public,
            message_count: 0,
        }
    }
}

impl Initialized<event::chat::private::Created> for Chat {
    fn init(_: &event::chat::private::Created) -> Self {
        Self {
            visibility: Visibility::Private,
            message_count: 0,
        }
    }
}

impl Sourced<event::message::Posted> for Chat {
    fn apply(&mut self, _: &event::message::Posted) {
        self.message_count += 1;
    }
}
