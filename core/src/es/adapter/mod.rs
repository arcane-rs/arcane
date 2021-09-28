//! [`Adapter`] definitions.

pub mod transformer;

use std::{
    fmt::{Debug, Formatter},
    pin::Pin,
    task::{Context, Poll},
};

use futures::{future, stream, Stream, StreamExt as _};
use pin_project::pin_project;
use ref_cast::RefCast;

#[doc(inline)]
pub use self::transformer::{strategy, Strategy, Transformer, WithStrategy};

/// Specifies result of [`Adapter`].
pub trait Returning {
    /// Error of this [`Adapter`].
    type Error;

    /// Converted [`Event`].
    ///
    /// [`Event`]: crate::es::Event
    type Transformed;
}

/// Facility to convert [`Event`]s.
/// Typical use cases include (but are not limited to):
///
/// - [`Skip`]ping unused [`Event`]s;
/// - Transforming (ex: from one [`Version`] to another);
/// - [`Split`]ting existing [`Event`]s into more granular ones.
///
/// Usually provided as blanket impl, so you shouldn't implement it manually.
/// For that you'll need to implement [`Returning`] to specify transformation
/// result and [`WithStrategy`] for every [`VersionedEvent`] which is part of
/// transformed [`Event`]. And as long as [`Event`] is implemented via derive
/// macro you should be good to go.
///
/// # Example
///
/// ```rust
/// # #![feature(generic_associated_types)]
/// #
/// # use std::convert::Infallible;
/// #
/// # use arcana::es::{
/// #     adapter::{self, strategy},
/// #     Event, EventAdapter as _, VersionedEvent,
/// # };
/// # use derive_more::From;
/// # use futures::{stream, TryStreamExt as _};
/// #
/// #[derive(Clone, Copy, Debug, PartialEq, VersionedEvent)]
/// #[event(name = "chat", version = 1)]
/// struct ChatEvent;
///
/// #[derive(Clone, Copy, Debug, PartialEq, VersionedEvent)]
/// #[event(name = "file", version = 2)]
/// struct FileEvent;
///
/// // Some outdated Event.
/// #[derive(Clone, Copy, Debug, PartialEq, VersionedEvent)]
/// #[event(name = "file", version = 1)]
/// struct FileEventV1;
///
/// // Repository-level Event, which is loaded from some Event Store and
/// // includes legacy Events.
/// #[derive(Clone, Copy, Debug, Event, PartialEq, From)]
/// enum RepositoryEvent {
///     FileV1(FileEventV1),
///     File(FileEvent),
///     Chat(ChatEvent),
/// }
///
/// // Actual Event we want to transform RepositoryEvent into
/// #[derive(Clone, Copy, Debug, Event, From, PartialEq)]
/// enum FileDomainEvent {
///     File(FileEvent),
/// }
///
/// #[derive(Clone, Copy)]
/// struct Adapter;
///
/// impl adapter::Returning for Adapter {
///     type Error = Infallible;
///     type Transformed = FileDomainEvent;
/// }
///
/// impl adapter::WithStrategy<FileEvent> for Adapter {
///     type Strategy = strategy::AsIs;
/// }
///
/// impl adapter::WithStrategy<FileEventV1> for Adapter {
///     type Strategy = strategy::Into<FileEvent>;
/// }
///
/// impl adapter::WithStrategy<ChatEvent> for Adapter {
///     type Strategy = strategy::Skip;
/// }
///
/// # let assertion = async {
/// let events = stream::iter::<[RepositoryEvent; 3]>([
///     FileEventV1.into(),
///     FileEvent.into(),
///     ChatEvent.into(),
/// ]);
///
/// let transformed = Adapter
///     .transform_all(events, &())
///     .try_collect::<Vec<_>>()
///     .await
///     .unwrap();
///
/// assert_eq!(transformed, vec![FileEvent.into(), FileEvent.into()]);
/// # };
/// #
/// # futures::executor::block_on(assertion);
/// #
/// # impl From<FileEventV1> for FileEvent {
/// #     fn from(_: FileEventV1) -> Self {
/// #         Self
/// #     }
/// # }
/// ```
///
/// [`Error`]: Self::Error
/// [`Event`]: crate::es::Event
/// [`Skip`]: transformer::strategy::Skip
/// [`Split`]: transformer::strategy::Split
/// [`Transformed`]: Self::Transformed
/// [`Version`]: crate::es::event::Version
/// [`VersionedEvent`]: crate::es::VersionedEvent
pub trait Adapter<Events, Ctx: ?Sized> {
    /// Error of this [`Adapter`].
    type Error;

    /// Converted [`Event`].
    ///
    /// [`Event`]: crate::es::Event
    type Transformed;

    /// [`Stream`] of [`Transformed`] [`Event`]s.
    ///
    /// [`Event`]: crate::es::Event
    /// [`Transformed`]: Self::Transformed
    type TransformedStream<'out>: Stream<
            Item = Result<
                <Self as Adapter<Events, Ctx>>::Transformed,
                <Self as Adapter<Events, Ctx>>::Error,
            >,
        > + 'out
    where
        Ctx: 'out,
        Events: 'out,
        Self: 'out;

    /// Converts all incoming [`Event`]s into [`Transformed`].
    ///
    /// [`Event`]: crate::es::Event
    /// [`Transformed`]: Self::Transformed
    fn transform_all<'me, 'ctx, 'out>(
        &'me self,
        events: Events,
        context: &'ctx Ctx,
    ) -> Self::TransformedStream<'out>
    where
        'me: 'out,
        'ctx: 'out;
}

impl<A, Events, Ctx> Adapter<Events, Ctx> for A
where
    A: Returning,
    Ctx: ?Sized,
    Events: Stream,
    Wrapper<A>: Transformer<Events::Item, Ctx>,
    A::Transformed:
        From<<Wrapper<A> as Transformer<Events::Item, Ctx>>::Transformed>,
    A::Error: From<<Wrapper<A> as Transformer<Events::Item, Ctx>>::Error>,
{
    type Error = <A as Returning>::Error;
    type Transformed = <A as Returning>::Transformed;
    type TransformedStream<'out>
    where
        Ctx: 'out,
        Events: 'out,
        Self: 'out,
    = TransformedStream<'out, Wrapper<A>, Events, Ctx>;

    fn transform_all<'me, 'ctx, 'out>(
        &'me self,
        events: Events,
        context: &'ctx Ctx,
    ) -> Self::TransformedStream<'out>
    where
        'me: 'out,
        'ctx: 'out,
    {
        TransformedStream::new(RefCast::ref_cast(self), events, context)
    }
}

/// Wrapper type for [`Adapter`] to satisfy orphan rules on [`Event`] derive
/// macro. Shouldn't be used manually.
///
/// [`Event`]: crate::es::Event
#[derive(Debug, RefCast)]
#[repr(transparent)]
pub struct Wrapper<A>(pub A);

impl<A> Returning for Wrapper<A>
where
    A: Returning,
{
    type Error = A::Error;
    type Transformed = A::Transformed;
}

#[pin_project]
/// [`Stream`] for [`Adapter`] blanket impl.
pub struct TransformedStream<'out, Adapter, Events, Ctx>
where
    Adapter: Transformer<Events::Item, Ctx>,
    Ctx: ?Sized,
    Events: Stream,
{
    #[pin]
    events: Events,
    #[pin]
    transformed_stream:
        AdapterTransformedStream<'out, Events::Item, Adapter, Ctx>,
    adapter: &'out Adapter,
    context: &'out Ctx,
}

impl<'out, Adapter, Events, Ctx> Debug
    for TransformedStream<'out, Adapter, Events, Ctx>
where
    Adapter: Debug + Transformer<Events::Item, Ctx>,
    Ctx: Debug + ?Sized,
    Events: Debug + Stream,
{
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TransformStream")
            .field("events", &self.events)
            .field("adapter", &self.adapter)
            .field("context", &self.context)
            .finish_non_exhaustive()
    }
}

type AdapterTransformedStream<'out, Event, Adapter, Ctx> = future::Either<
    <Adapter as Transformer<Event, Ctx>>::TransformedStream<'out>,
    stream::Empty<
        Result<
            <Adapter as Transformer<Event, Ctx>>::Transformed,
            <Adapter as Transformer<Event, Ctx>>::Error,
        >,
    >,
>;

impl<'out, Adapter, Events, Ctx> TransformedStream<'out, Adapter, Events, Ctx>
where
    Adapter: Transformer<Events::Item, Ctx>,
    Ctx: ?Sized,
    Events: Stream,
{
    fn new(adapter: &'out Adapter, events: Events, context: &'out Ctx) -> Self {
        Self {
            events,
            transformed_stream: stream::empty().right_stream(),
            adapter,
            context,
        }
    }
}

impl<'out, Adapter, Events, Ctx> Stream
    for TransformedStream<'out, Adapter, Events, Ctx>
where
    Ctx: ?Sized,
    Adapter: Transformer<Events::Item, Ctx> + Returning,
    Events: Stream,
    <Adapter as Returning>::Transformed:
        From<<Adapter as Transformer<Events::Item, Ctx>>::Transformed>,
    <Adapter as Returning>::Error:
        From<<Adapter as Transformer<Events::Item, Ctx>>::Error>,
{
    type Item = Result<
        <Adapter as Returning>::Transformed,
        <Adapter as Returning>::Error,
    >;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            let res =
                futures::ready!(this.transformed_stream.as_mut().poll_next(cx));
            if let Some(ev) = res {
                return Poll::Ready(Some(
                    ev.map(Into::into).map_err(Into::into),
                ));
            }

            let res = futures::ready!(this.events.as_mut().poll_next(cx));
            if let Some(event) = res {
                let new_stream =
                    Adapter::transform(*this.adapter, event, *this.context);
                this.transformed_stream.set(new_stream.left_stream());
            } else {
                return Poll::Ready(None);
            }
        }
    }
}
