use arcana::es::event;

#[derive(Debug, PartialEq, event::Versioned)]
#[event(name = "email.added", version = 3)]
pub struct Added {
    pub email: String,
}

#[derive(Debug, PartialEq, event::Versioned)]
#[event(name = "email.confirmed", version = 3)]
pub struct Confirmed {
    pub confirmed_by: String,
}

pub mod v2 {
    use super::event;

    #[derive(Debug, serde::Deserialize, PartialEq, event::Versioned)]
    #[event(name = "email.added_and_confirmed", version = 2)]
    pub struct AddedAndConfirmed {
        pub email: String,
        pub confirmed_by: Option<String>,
    }
}
