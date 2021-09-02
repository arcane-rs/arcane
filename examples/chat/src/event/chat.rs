use arcana::es::event;

pub mod private {
    use super::event;

    #[derive(Debug, event::Versioned)]
    #[event(name = "chat.private.created", version = 2)]
    pub struct Created;
}

pub mod public {
    use super::event;

    #[derive(Debug, event::Versioned)]
    #[event(name = "chat.public.created", version = 2)]
    pub struct Created;
}

pub mod v1 {
    use super::event;

    #[derive(Debug, event::Versioned)]
    #[event(name = "chat.created", version = 1)]
    pub struct Created;
}
