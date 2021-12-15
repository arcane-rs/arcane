use arcana::es::event::{Initialized, Sourced};

use crate::event;

#[derive(Debug, Eq, PartialEq)]
pub struct Email {
    pub value: String,
    pub confirmed_by: Option<String>,
}

impl Initialized<event::email::Added> for Email {
    fn init(event: &event::email::Added) -> Self {
        Self {
            value: event.email.clone(),
            confirmed_by: None,
        }
    }
}

impl Sourced<event::email::Confirmed> for Email {
    fn apply(&mut self, event: &event::email::Confirmed) {
        self.confirmed_by = Some(event.confirmed_by.clone());
    }
}
