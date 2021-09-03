use arcana::es::event;

#[derive(Debug, event::Versioned)]
#[event(name = "email.added", version = 2)]
pub struct Added {
    pub email: String,
}

#[derive(Debug, event::Versioned)]
#[event(name = "email.confirmed", version = 2)]
pub struct Confirmed {
    pub confirmed_by: String,
}

pub mod v1 {
    use super::event;

    #[derive(Debug, event::Versioned)]
    #[event(name = "email.added_and_confirmed", version = 1)]
    pub struct AddedAndConfirmed {
        pub email: String,
        pub confirmed_by: Option<String>,
    }
}
