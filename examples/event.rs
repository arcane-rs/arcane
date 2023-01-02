use arcane::es::event::{Event, reflect, Meta, Initialized, Sourced, Sourcing};

#[derive(Event)]
#[event(name = "chat.created", revision = 1)]
struct ChatCreated;

// TODO: Use no revision when `#[derive(Event)]` on enums supports it.
#[derive(Event)]
#[event(name = "message.posted", revision = 1)]
struct MessagePosted;

#[derive(Event)]
enum ChatEvent {
    #[event(init)]
    Created(ChatCreated),
    MessagePosted(MessagePosted),
}

#[derive(Event)]
enum MessageEvent {
    #[event(init)]
    MessagePosted(MessagePosted),
}

#[derive(Event)]
enum AnyEvent {
    Chat(ChatEvent),
    Message(MessageEvent),
}

#[derive(Debug, Eq, PartialEq)]
struct Chat {
    message_count: usize,
}

impl Initialized<ChatCreated> for Chat {
    fn init(_: &ChatCreated) -> Self {
        Self { message_count: 0 }
    }
}

impl Sourced<MessagePosted> for Chat {
    fn apply(&mut self, _: &MessagePosted) {
        self.message_count += 1;
    }
}

#[derive(Debug, Eq, PartialEq)]
struct Message;

impl Initialized<MessagePosted> for Message {
    fn init(_: &MessagePosted) -> Self {
        Self
    }
}

fn assert_meta<E: reflect::Meta>(expected: &[Meta]) {
    let actual = E::META;

    assert_eq!(actual.len(), expected.len());

    for (actual, expected) in actual.iter().zip(expected.iter()) {
        assert_eq!(actual.name, expected.name);
        assert_eq!(actual.revision, expected.revision);
    }
}

fn main() {
    assert_meta::<ChatCreated>(&[Meta {
        name: "chat.created",
        revision: Revision::try_new(1).unwrap(),
    }]);

    assert_meta::<MessagePosted>(&[Meta {
        name: "message.posted",
        revision: Revision::try_new(1).unwrap(),
    }]);

    assert_meta::<ChatEvent>(&[
        Meta {
            name: "chat.created",
            revision: Revision::try_new(1).unwrap(),
        },
        Meta {
            name: "message.posted",
            revision: Revision::try_new(1).unwrap(),
        },
    ]);

    assert_meta::<MessageEvent>(&[Meta {
        name: "message.posted",
        revision: Revision::try_new(1).unwrap(),
    }]);

    assert_meta::<AnyEvent>(&[
        Meta {
            name: "chat.created",
            revision: Revision::try_new(1).unwrap(),
        },
        Meta {
            name: "message.posted",
            revision: Revision::try_new(1).unwrap(),
        },
        Meta {
            name: "message.posted",
            revision: Revision::try_new(1).unwrap(),
        },
    ]);

    let mut chat = Option::<Chat>::None;
    let mut message = Option::<Message>::None;

    let ev = ChatEvent::Created(ChatCreated.into());
    chat.apply(&ev);
    assert_eq!(ev.name(), "chat.created");
    assert_eq!(chat, Some(Chat { message_count: 0 }));

    let ev = ChatEvent::MessagePosted(MessagePosted);
    chat.apply(&ev);
    assert_eq!(ev.name(), "message.posted");
    assert_eq!(chat, Some(Chat { message_count: 1 }));

    let ev: &dyn Sourcing<Option<Chat>> = &ev;
    chat.apply(ev);
    assert_eq!(chat, Some(Chat { message_count: 2 }));

    let ev = MessageEvent::MessagePosted(MessagePosted.into());
    message.apply(&ev);
    assert_eq!(ev.name(), "message.posted");
    assert_eq!(message, Some(Message));

    let ev = AnyEvent::Chat(ChatEvent::Created(ChatCreated.into()));
    assert_eq!(ev.name(), "chat.created");

    let ev =
        AnyEvent::Message(MessageEvent::MessagePosted(MessagePosted.into()));
    assert_eq!(ev.name(), "message.posted");
}
