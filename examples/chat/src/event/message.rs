use arcana::es::event;

#[derive(Debug, PartialEq, event::Versioned)]
#[event(name = "message.posted", version = 1)]
pub struct Posted;
