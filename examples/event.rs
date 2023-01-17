use arcane::es::event::{
    reflect, Event, Initialized, Name, Revisable, Sourced, Sourcing, Raw,
    Version,
};

#[derive(Event)]
#[event(name = "chat.created")]
struct ChatCreated;

#[derive(Clone, Copy, Debug, Event, PartialEq)]
#[event(name = "message.posted", rev = 1)]
struct MessagePosted;

#[derive(Event)]
enum ChatEvent {
    #[event(init)]
    Created(ChatCreated),
    MessagePosted(MessagePosted),
}

#[derive(Clone, Copy, Debug, Event, PartialEq)]
#[event(rev)]
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

fn assert_names<E>(expected: impl AsRef<[Name]>)
where
    E: reflect::Static,
{
    let expected = expected.as_ref();

    assert_eq!(E::NAMES.len(), expected.len());
    for (actual, expected) in E::NAMES.iter().zip(expected) {
        assert_eq!(actual, expected);
    }
}

fn assert_revisions<E>(expected: impl AsRef<[Version]>)
where
    E: reflect::Concrete<Revision = Version>,
{
    let expected = expected.as_ref();

    assert_eq!(E::REVISIONS.len(), expected.len());
    for (actual, expected) in E::REVISIONS.iter().zip(expected) {
        assert_eq!(actual, expected);
    }
}

fn main() {
    assert_names::<ChatCreated>(["chat.created"]);
    assert_names::<MessagePosted>(["message.posted"]);
    assert_names::<ChatEvent>(["chat.created", "message.posted"]);
    assert_names::<MessageEvent>(["message.posted"]);
    assert_names::<AnyEvent>([
        "chat.created",
        "message.posted",
        "message.posted",
    ]);

    assert_revisions::<MessagePosted>([Version::try_new(1).unwrap()]);

    let ev = MessageEvent::MessagePosted(MessagePosted);
    let raw = Raw::from(ev.clone());
    assert_eq!(raw.name, ev.name());
    assert_eq!(raw.revision, ev.revision());
    assert_eq!(raw.event, ev);
    let raw_ev = MessageEvent::try_from(raw).unwrap();
    assert_eq!(raw_ev, ev);

    let ev = MessagePosted;
    let raw = Raw::from(ev.clone());
    assert_eq!(raw.name, ev.name());
    assert_eq!(raw.revision, ev.revision());
    assert_eq!(raw.event, ev);
    let raw_ev = MessagePosted::try_from(raw).unwrap();
    assert_eq!(raw_ev, ev);

    let mut chat = Option::<Chat>::None;
    let mut message = Option::<Message>::None;

    let ev = ChatEvent::Created(ChatCreated);
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

    let ev = MessageEvent::MessagePosted(MessagePosted);
    message.apply(&ev);
    assert_eq!(ev.name(), "message.posted");
    assert_eq!(message, Some(Message));
    assert_eq!(ev.revision(), Version::try_new(1).unwrap());

    let ev = AnyEvent::Chat(ChatEvent::Created(ChatCreated.into()));
    assert_eq!(ev.name(), "chat.created");

    let ev =
        AnyEvent::Message(MessageEvent::MessagePosted(MessagePosted));
    assert_eq!(ev.name(), "message.posted");
}
