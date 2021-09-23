pub mod chat;
pub mod email;
pub mod message;

use arcana::{
    es,
    es::adapter::{self, transformer::Transformer, And},
};
use derive_more::From;
use futures::stream::{LocalBoxStream, StreamExt as _};

use crate::event;

#[derive(Debug, es::Event, From)]
pub enum Event {
    Chat(ChatEvent),
    Message(MessageEvent),
    Email(EmailEvent),
}

impl<A> Transformer<Event> for adapter::Wrapper<A>
    where
        A: adapter::WithError,
        Self: Transformer<ChatEvent>
        + Transformer<MessageEvent>
        + Transformer<EmailEvent>,
        A::Error: From<<Self as Transformer<ChatEvent>>::Error>
        + From<<Self as Transformer<MessageEvent>>::Error>
        + From<<Self as Transformer<EmailEvent>>::Error>
        + 'static,
        A::Transformed: From<<Self as Transformer<ChatEvent>>::Transformed>
        + From<<Self as Transformer<MessageEvent>>::Transformed>
        + From<<Self as Transformer<EmailEvent>>::Transformed>
        + 'static,
{
    type Context<Impl> = And<
        <Self as Transformer<ChatEvent>>::Context<Impl>,
        And<
            <Self as Transformer<MessageEvent>>::Context<Impl>,
            <Self as Transformer<EmailEvent>>::Context<
                Impl,
            >,
        >,
    >;

    type Error = A::Error;

    type Transformed = A::Transformed;

    type TransformedStream<'out, Ctx: 'static> =
    LocalBoxStream<'out, Result<A::Transformed, A::Error>>;

    fn transform<'me, 'ctx, 'out, Ctx>(
        &'me self,
        event: Event,
        context: &'ctx Ctx,
    ) -> Self::TransformedStream<'out, Ctx>
        where
            'me: 'out,
            'ctx: 'out,
            Ctx: 'static,
    {
        match event {
            Event::Chat(ev) => {
                Transformer::<ChatEvent>::transform(self, ev, context)
                    .map(|res| res.map(Into::into).map_err(Into::into))
                    .boxed_local()
            }
            Event::Message(ev) => {
                Transformer::<MessageEvent>::transform(
                    self, ev, context,
                )
                    .map(|res| res.map(Into::into).map_err(Into::into))
                    .boxed_local()
            }
            Event::Email(ev) => {
                Transformer::<EmailEvent>::transform(
                    self, ev, context,
                )
                    .map(|res| res.map(Into::into).map_err(Into::into))
                    .boxed_local()
            }
        }
    }
}

#[derive(Debug, es::Event, From)]
pub enum ChatEvent {
    Created(event::chat::v1::Created),
    PublicCreated(event::chat::public::Created),
    PrivateCreated(event::chat::private::Created),
}

impl<A> Transformer<ChatEvent> for adapter::Wrapper<A>
    where
        A: adapter::WithError,
        Self: Transformer<event::chat::v1::Created>
        + Transformer<event::chat::public::Created>
        + Transformer<event::chat::private::Created>,
        A::Error: From<<Self as Transformer<event::chat::v1::Created>>::Error>
        + From<<Self as Transformer<event::chat::public::Created>>::Error>
        + From<<Self as Transformer<event::chat::private::Created>>::Error>
        + 'static,
        A::Transformed: From<<Self as Transformer<event::chat::v1::Created>>::Transformed>
        + From<<Self as Transformer<event::chat::public::Created>>::Transformed>
        + From<<Self as Transformer<event::chat::private::Created>>::Transformed>
        + 'static,
{
    type Context<Impl> = And<
        <Self as Transformer<event::chat::v1::Created>>::Context<Impl>,
        And<
            <Self as Transformer<event::chat::public::Created>>::Context<Impl>,
            <Self as Transformer<event::chat::private::Created>>::Context<
                Impl,
            >,
        >,
    >;

    type Error = A::Error;

    type Transformed = A::Transformed;

    type TransformedStream<'out, Ctx: 'static> =
    LocalBoxStream<'out, Result<A::Transformed, A::Error>>;

    fn transform<'me, 'ctx, 'out, Ctx>(
        &'me self,
        event: ChatEvent,
        context: &'ctx Ctx,
    ) -> Self::TransformedStream<'out, Ctx>
        where
            'me: 'out,
            'ctx: 'out,
            Ctx: 'static,
    {
        match event {
            ChatEvent::Created(ev) => {
                Transformer::<event::chat::v1::Created>::transform(self, ev, context)
                    .map(|res| res.map(Into::into).map_err(Into::into))
                    .boxed_local()
            }
            ChatEvent::PublicCreated(ev) => {
                Transformer::<event::chat::public::Created>::transform(
                    self, ev, context,
                )
                    .map(|res| res.map(Into::into).map_err(Into::into))
                    .boxed_local()
            }
            ChatEvent::PrivateCreated(ev) => {
                Transformer::<event::chat::private::Created>::transform(
                    self, ev, context,
                )
                    .map(|res| res.map(Into::into).map_err(Into::into))
                    .boxed_local()
            }
        }
    }
}

#[derive(Debug, es::Event, From)]
pub enum MessageEvent {
    Posted(event::message::Posted),
}

impl<A> Transformer<MessageEvent> for adapter::Wrapper<A>
    where
        A: adapter::WithError,
        Self: Transformer<event::message::Posted>,
        A::Error: From<<Self as Transformer<event::message::Posted>>::Error>
        + 'static,
        A::Transformed: From<<Self as Transformer<event::message::Posted>>::Transformed>
        + 'static,
{
    type Context<Impl> =
        <Self as Transformer<event::message::Posted>>::Context<Impl>;

    type Error = A::Error;

    type Transformed = A::Transformed;

    type TransformedStream<'out, Ctx: 'static> =
    LocalBoxStream<'out, Result<A::Transformed, A::Error>>;

    fn transform<'me, 'ctx, 'out, Ctx>(
        &'me self,
        event: MessageEvent,
        context: &'ctx Ctx,
    ) -> Self::TransformedStream<'out, Ctx>
        where
            'me: 'out,
            'ctx: 'out,
            Ctx: 'static,
    {
        match event {
            MessageEvent::Posted(ev) => {
                Transformer::<event::message::Posted>::transform(self, ev, context)
                    .map(|res| res.map(Into::into).map_err(Into::into))
                    .boxed_local()
            }
        }
    }
}

#[derive(Debug, es::Event, From)]
pub enum EmailEvent {
    Added(event::email::Added),
    Confirmed(event::email::Confirmed),
    AddedAndConfirmed(event::email::v1::AddedAndConfirmed),
}

impl<A> Transformer<EmailEvent> for adapter::Wrapper<A>
where
    A: adapter::WithError,
    Self: Transformer<event::email::Added>
        + Transformer<event::email::Confirmed>
        + Transformer<event::email::v1::AddedAndConfirmed>,
    A::Error: From<<Self as Transformer<event::email::Added>>::Error>
        + From<<Self as Transformer<event::email::Confirmed>>::Error>
        + From<<Self as Transformer<event::email::v1::AddedAndConfirmed>>::Error>
        + 'static,
    A::Transformed: From<<Self as Transformer<event::email::Added>>::Transformed>
        + From<<Self as Transformer<event::email::Confirmed>>::Transformed>
        + From<<Self as Transformer<event::email::v1::AddedAndConfirmed>>::Transformed>
        + 'static,
{
    type Context<Impl> = And<
        <Self as Transformer<event::email::Added>>::Context<Impl>,
        And<
            <Self as Transformer<event::email::Confirmed>>::Context<Impl>,
            <Self as Transformer<event::email::v1::AddedAndConfirmed>>::Context<
                Impl,
            >,
        >,
    >;

    type Error = A::Error;

    type Transformed = A::Transformed;

    type TransformedStream<'out, Ctx: 'static> =
        LocalBoxStream<'out, Result<A::Transformed, A::Error>>;

    fn transform<'me, 'ctx, 'out, Ctx>(
        &'me self,
        event: EmailEvent,
        context: &'ctx Ctx,
    ) -> Self::TransformedStream<'out, Ctx>
    where
        'me: 'out,
        'ctx: 'out,
        Ctx: 'static,
    {
        match event {
            EmailEvent::Added(ev) => {
                Transformer::<event::email::Added>::transform(self, ev, context)
                    .map(|res| res.map(Into::into).map_err(Into::into))
                    .boxed_local()
            }
            EmailEvent::Confirmed(ev) => {
                Transformer::<event::email::Confirmed>::transform(
                    self, ev, context,
                )
                .map(|res| res.map(Into::into).map_err(Into::into))
                .boxed_local()
            }
            EmailEvent::AddedAndConfirmed(ev) => {
                Transformer::<event::email::v1::AddedAndConfirmed>::transform(
                    self, ev, context,
                )
                .map(|res| res.map(Into::into).map_err(Into::into))
                .boxed_local()
            }
        }
    }
}
