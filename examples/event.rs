use arcana::es::{event, Event};

#[derive(event::Versioned)]
#[event(name = "chat.created", version = 1)]
struct ChatCreated;

#[derive(event::Versioned)]
#[event(name = "message.posted", version = 1)]
struct MessagePosted;

#[derive(Event)]
enum ChatEvent {
    Created(event::Initial<ChatCreated>),
    MessagePosted(MessagePosted),
}

#[derive(Event)]
enum MessageEvent {
    MessagePosted(event::Initial<MessagePosted>),
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
