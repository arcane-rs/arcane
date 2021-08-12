use ref_cast::RefCast;

pub trait Event {
    fn fqn(&self) -> &'static str;
    fn version(&self) -> u16;
}
pub trait VersionedEvent {
    fn fqn() -> &'static str;
    fn version() -> u16;
}

pub trait Sourced<Ev: ?Sized> {
    fn apply(&mut self, event: &Ev);
}
impl<Ev: Event + ?Sized, Agg: Sourced<Ev>> Sourced<Ev> for Option<Agg> {
    fn apply(&mut self, event: &Ev) {
        if let Some(agg) = self {
            agg.apply(event)
        }
    }
}

pub trait Initialized<Ev: ?Sized> {
    fn init(event: &Ev) -> Self;
}
#[derive(RefCast)]
#[repr(transparent)]
pub struct Initial<Ev: ?Sized>(Ev);
impl<Ev: Event + ?Sized, Agg: Initialized<Ev>> Sourced<Initial<Ev>> for Option<Agg> {
    fn apply(&mut self, event: &Initial<Ev>) {
        *self = Some(Agg::init(&event.0))
    }
}

//////////////////

pub struct User {
    id: u8,
}

pub struct UserCreated {
    id: u8,
}
impl VersionedEvent for UserCreated {
    fn fqn() -> &'static str {
        "user.created"
    }
    fn version() -> u16 {
        1
    }
}
impl Event for UserCreated {
    fn fqn(&self) -> &'static str {
        <Self as VersionedEvent>::fqn()
    }
    fn version(&self) -> u16 {
        <Self as VersionedEvent>::version()
    }
}
impl Initialized<UserCreated> for User {
    fn init(ev: &UserCreated) -> Self {
        Self {
            id: ev.id,
        }
    }
}

pub struct UserDeleted {
    id: u8,
}
impl Event for UserDeleted {}
impl Sourced<UserDeleted> for User {
    fn apply(&mut self, event: &UserDeleted) {
        self.id = event.id
    }
}

pub enum UserEvent {
    // #[initial]
    Created(UserCreated),
    Deleted(UserDeleted),
}
impl Event for UserEvent {}
impl Sourced<UserEvent> for Option<User> {
    fn apply(&mut self, event: &UserEvent) {
        match event {
            UserEvent::Created(ev) => self.apply(<Initial<UserCreated> as RefCast>::ref_cast(ev)),
            UserEvent::Deleted(ev) => self.apply(ev),
        }
    }
}

//////////////////

fn main() {}
