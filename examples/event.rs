use arcana::es::{event, Event};

#[derive(event::Versioned)]
#[event(name = "chat", version = 1)]
struct ChatEvent;

#[derive(event::Versioned)]
#[event(name = "file", version = 1)]
struct FileEvent;

#[derive(Event)]
enum AnyEvent {
    Chat(ChatEvent),
    File(FileEvent),
}

fn main() {
    let ev = AnyEvent::Chat(ChatEvent);
    assert_eq!(ev.name(), "chat");

    let ev = AnyEvent::File(FileEvent);
    assert_eq!(ev.name(), "file");
}
