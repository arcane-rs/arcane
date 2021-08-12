use async_trait::async_trait;
use arcana::{
    build::{And, Handler, New, Of, With},
    cqrs::{aggregate, Aggregate, Command, CommandHandler},
    es::{
        event, AggregateEvent, DomainEvent, Event, EventInitialized,
        EventSourced,
    },
};
use derive_more::From;
use std::marker::PhantomData;

#[derive(Clone, Debug)]
pub struct Chat {
    pub id: u64,
    pub name: String,
}
impl Aggregate for Chat {
    type Id = u64;
    fn type_name(&self) -> aggregate::TypeName {
        "chat"
    }
    fn id(&self) -> &Self::Id {
        &self.id
    }
}
impl EventInitialized<ChatCreated> for Chat {
    fn init(ev: &ChatCreated) -> Self {
        Self {
            id: ev.id,
            name: ev.name.clone(),
        }
    }
}
impl EventSourced<ChatNameUpdated> for Chat {
    fn apply(&mut self, ev: &ChatNameUpdated) {
        self.name = ev.name.clone();
    }
}

#[derive(Clone, Debug)]
pub struct ChatCreated {
    pub id: u64,
    pub name: String,
}
impl Event for ChatCreated {
    fn type_name(&self) -> event::TypeName {
        "chat.created"
    }
}

#[derive(Clone, Debug)]
pub struct ChatNameUpdated {
    pub id: u64,
    pub name: String,
}
impl Event for ChatNameUpdated {
    fn type_name(&self) -> event::TypeName {
        "chat.name.updated"
    }
}

#[derive(Clone, Debug, From)]
pub enum ChatEvent {
    Created(ChatCreated),
    NameUpdated(ChatNameUpdated),
}
impl Event for ChatEvent {
    fn type_name(&self) -> event::TypeName {
        match self {
            Self::Created(ev) => ev.type_name(),
            Self::NameUpdated(ev) => ev.type_name(),
        }
    }
}
impl AggregateEvent for ChatEvent {
    type Aggregate = Chat;

    fn type_names() -> &'static [event::TypeName] {
        &["chat.created", "chat.name.updated"]
    }
}
impl EventSourced<ChatEvent> for Chat {
    fn apply(&mut self, ev: &ChatEvent) {
        match ev {
            ChatEvent::Created(ev) => self.apply(ev),
            ChatEvent::NameUpdated(ev) => self.apply(ev),
        }
    }
}

#[derive(Clone, Debug)]
pub struct Message {
    pub id: u64,
    pub text: String,
    pub is_deleted: bool,
}
impl Aggregate for Message {
    type Id = u64;
    fn type_name(&self) -> aggregate::TypeName {
        "message"
    }
    fn id(&self) -> &Self::Id {
        &self.id
    }
}
impl EventInitialized<MessagePosted> for Message {
    fn init(ev: &MessagePosted) -> Self {
        Self {
            id: ev.id,
            text: ev.text.clone(),
            is_deleted: false,
        }
    }
}
impl EventSourced<MessageDeleted> for Message {
    fn apply(&mut self, _: &MessageDeleted) {
        self.is_deleted = true;
    }
}

#[derive(Clone, Debug)]
pub struct MessagePosted {
    pub id: u64,
    pub text: String,
}
impl Event for MessagePosted {
    fn type_name(&self) -> event::TypeName {
        "message.posted"
    }
}

#[derive(Clone, Debug)]
pub struct MessageDeleted {
    pub id: u64,
}
impl Event for MessageDeleted {
    fn type_name(&self) -> event::TypeName {
        "message.deleted"
    }
}

#[derive(Clone, Debug, From)]
pub enum MessageEvent {
    Posted(MessagePosted),
    Deleted(MessageDeleted),
}
impl Event for MessageEvent {
    fn type_name(&self) -> event::TypeName {
        match self {
            Self::Posted(ev) => ev.type_name(),
            Self::Deleted(ev) => ev.type_name(),
        }
    }
}
impl AggregateEvent for MessageEvent {
    type Aggregate = Message;

    fn type_names() -> &'static [event::TypeName] {
        &["message.posted", "message.deleted"]
    }
}
impl EventSourced<MessageEvent> for Message {
    fn apply(&mut self, ev: &MessageEvent) {
        match ev {
            MessageEvent::Posted(ev) => self.apply(ev),
            MessageEvent::Deleted(ev) => self.apply(ev),
        }
    }
}

#[derive(Clone, Debug, From)]
pub enum AnyEvent {
    Chat(ChatEvent),
    Message(MessageEvent),
}
impl Event for AnyEvent {
    fn type_name(&self) -> event::TypeName {
        match self {
            Self::Chat(ev) => ev.type_name(),
            Self::Message(ev) => ev.type_name(),
        }
    }
}
impl DomainEvent for AnyEvent {}
impl EventSourced<AnyEvent> for <ChatEvent as AggregateEvent>::Aggregate {
    fn apply(&mut self, ev: &AnyEvent) {
        if let AnyEvent::Chat(ev) = ev {
            self.apply(ev)
        }
    }
}
impl EventSourced<AnyEvent> for <MessageEvent as AggregateEvent>::Aggregate {
    fn apply(&mut self, ev: &AnyEvent) {
        if let AnyEvent::Message(ev) = ev {
            self.apply(ev)
        }
    }
}

#[derive(Clone, Debug)]
pub struct CreateChat {
    pub name: String,
}
impl Command for CreateChat {
    type Aggregate = Chat;

    fn aggregate_id(&self) -> Option<&u64> {
        None
    }
}

#[async_trait(?Send)]
pub trait Repository {
    async fn check_name_exists(&self, name: &str) -> bool;
}

pub type Meta = ();

#[async_trait(?Send)]
impl<T> CommandHandler<CreateChat> for Handler<New<Chat>, With<T>>
where
    T: Repository,
{
    type Result = Option<ChatCreated>;

    async fn handle(&mut self, cmd: CreateChat) -> Self::Result {
        let repo = self.context();
        if !repo.check_name_exists(&cmd.name).await {
            return None;
        }
        Some(ChatCreated {
            id: 45,
            name: cmd.name,
        })
    }
}

/*
pub struct Command<A, B>(A, B);

pub type CreateChat = Command<New<Chat>, With<data::chat::Create>>;

pub type UpdateChatName =
    Command<Chat, With<(data::chat::UpdateName, AndMeta<Meta>)>>;

pub type UpdateChatName =
    Command<data::chat::UpdateName, Handler<Chat>>;


pub type IdempotentCreateChat = Command<
    New<Chat>,
    WhenAbsent<With<(data::chat::Create, AndMeta<Common>)>>,
>;
pub type IdempotentCreateChat = Command<
    WhenAbsent<data::chat::Create>,
    Handler<New<Chat>>,
>;

1. Extract ID from Command
2. If None:
    2.1. Run CommandHandler on None value of Option<Aggregate>
3. If Some:
    3.1. Load Aggregate from snapshot
    3.2. If None:
        3.2.1. Run CommandHandler on None value of Option<Aggregate>
    3.3. If Some:
        3.3.1. Run CommandHandler on Some value of Option<Aggregate>

Create new chat (not idempotent) expects `Chat` as result:
1. Command returns None aggregate_id
2. Run command on `Option<Chat>` being a `None` as CommandHandler
3. It produces Event that initializes Chat
4. Event is applied to Option<Chat> producing Some(Chat),
5. Chat is persisted and returned

Create new file (idempotent) expects `File` as result:
1. Command returns Some(aggregate_id)
2. Repository loads Option<File> aggregate
3. Run command on Option<File> as CommandHandler
4. Option<File> is some => return no events
5. Option<File> is none => return event that initializes File
6. Option<Event> is applied to Option<File> producing Some(File)
7. File is persisted and returned

Update chat name (idempotent) expect Option<Chat> as result:
1. Command returns Some(aggregate_id)
2. Repository loads Option<Chat> aggregate
3. If none => returns None
4. Run command on Some(Chat) as CommandHandler
5. If name the same => returns no events
6. If name changed => returns event that updates name
7. Option<Chat> is applied to Option<Chat> producing Option<Chat>
8. If Some(chat) => it's persisted
9. Return Option<Chat>

Abstract logic:
1. Commands returns Some(aggreage_id)
2. If Some => load Option<Aggregate>
3. If None => initialize None as Option<Aggregate>
4. Executing command on Option<Aggregate> produces Vec<Events>
5. Vec<Events> is not empty => applied to Option<Aggregate>
6. If Option<Aggregate> is some => it's persisted (no persistence required if no-op previously)
7. Option<Aggregate> is returned (panic if unpack to Aggregate, but better type error)

pub type IdempotentCreateChat = Command<
    Option<Chat>,
    WhenAbsent<With<data::chat::Create>,
    And<Meta, And<Extract<dyn Context>>>>,
>;
pub type IdempotentCreateChat = Command<
    Handler<New<Chat>, With<dyn Context, And<Something>>>,
    WhenAbsent<With<data::chat::Create, And<Meta>>>,
>;


 */

fn main() {}

// TargetedCommand

// impl CommandHandler for Absent<Chat>
// type Result = ??; // should be          // should require IntoJust<T>

// impl CommandHandler for Existing<Chat>
// type Result = ??; // can be optional    // should require IntoMaybe<T>

// impl CommandHandler for Option<Chat>
// type Result = ??; // cab be optional    // should require IntoMaybe<T>

// impl Event for Just<E: Event>
// impl Event for Option<ChatCreated>
