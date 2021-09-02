use arcana::es::event::{self, Event, Initial};

#[derive(event::Versioned)]
#[event(name = "chat.created", version = 1)]
struct ChatCreated;

#[derive(event::Versioned)]
#[event(name = "message.posted", version = 1)]
struct MessagePosted;

#[derive(Event)]
enum ChatEvent {
    Created(Initial<ChatCreated>),
    MessagePosted(MessagePosted),
}

impl<T> event::Sourced<ChatEvent> for Option<T>
where Self: event::Sourced<Initial<ChatCreated>> +
            event::Sourced<MessagePosted>
{
    fn apply(&mut self, event: &ChatEvent) {
        unimplemented!()
    }
}

#[derive(Event)]
enum MessageEvent {
    MessagePosted(Initial<MessagePosted>),
}

#[derive(Event)]
enum AnyEvent {
    Chat(ChatEvent),
    Message(MessageEvent),
}

fn main() {
    let ev = ChatEvent::Created(ChatCreated.into());
    assert_eq!(ev.name(), "chat.created");

    let ev = ChatEvent::MessagePosted(MessagePosted);
    assert_eq!(ev.name(), "message.posted");

    let ev = MessageEvent::MessagePosted(MessagePosted.into());
    assert_eq!(ev.name(), "message.posted");

    let ev = AnyEvent::Chat(ChatEvent::Created(ChatCreated.into()));
    assert_eq!(ev.name(), "chat.created");

    let ev =
        AnyEvent::Message(MessageEvent::MessagePosted(MessagePosted.into()));
    assert_eq!(ev.name(), "message.posted");
}
