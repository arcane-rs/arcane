use arcana::es::event::Initialized;

use crate::event::message::Posted;

#[derive(Debug, Eq, PartialEq)]
pub struct Message;

impl Initialized<Posted> for Message {
    fn init(_: &Posted) -> Self {
        Self
    }
}
