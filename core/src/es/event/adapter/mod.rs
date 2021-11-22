//! [`Adapter`] definitions.

pub mod transformer;

use std::{
    borrow::Borrow,
    fmt::{Debug, Formatter},
    pin::Pin,
    task,
};

use futures::{future, stream, Stream, StreamExt as _};
use pin_project::pin_project;
use ref_cast::RefCast;

use crate::spell;

#[doc(inline)]
pub use self::transformer::{
    strategy, Adapt, Strategy, Transformer, Types as TransformerTypes,
};

/// Specifies result of [`Adapter`].
///
/// Consider using [`Adapter`] derive macro instead of implementing this
/// manually.
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
/// result and [`Adapt`] for every [`VersionedEvent`] which is part of
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
/// #     event::adapter::{self, strategy},
/// #     Event, EventAdapter, VersionedEvent,
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
/// #[derive(Clone, Copy, Debug, Event, From, PartialEq)]
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
/// #[derive(Clone, Copy, EventAdapter)]
/// #[adapter(into = FileDomainEvent)]
/// struct Adapter;
///
/// impl adapter::Adapt<FileEvent> for Adapter {
///     type Strategy = strategy::AsIs;
/// }
///
/// impl adapter::Adapt<FileEventV1> for Adapter {
///     type Strategy = strategy::Into<FileEvent>;
/// }
///
/// impl adapter::Adapt<ChatEvent> for Adapter {
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
/// In case some of your [`Strategies`] are [`Custom`] or manual impl with
/// non-`()` [`Strategy::Context`], provided `context` should be able to be
/// [`Borrow`]ed as `dyn Trait`.
///
/// ```rust
/// # #![feature(generic_associated_types)]
/// #
/// # use std::{borrow::Borrow, convert::Infallible};
/// #
/// # use arcana::{
/// #     es::{
/// #         event::adapter::{self, strategy},
/// #         Event, EventAdapter, VersionedEvent,
/// #     },
/// #     spell,
/// # };
/// # use derive_more::From;
/// # use futures::{stream, TryStreamExt as _};
/// #
/// # #[derive(Clone, Copy, Debug, PartialEq, VersionedEvent)]
/// # #[event(name = "chat", version = 1)]
/// # struct ChatEvent;
/// #
/// # #[derive(Clone, Copy, Debug, PartialEq, VersionedEvent)]
/// # #[event(name = "file", version = 2)]
/// # struct FileEvent;
/// #
/// # // Some outdated Event.
/// # #[derive(Clone, Copy, Debug, PartialEq, VersionedEvent)]
/// # #[event(name = "file", version = 1)]
/// # struct FileEventV1;
/// #
/// # // Repository-level Event, which is loaded from some Event Store and
/// # // includes legacy Events.
/// # #[derive(Clone, Copy, Debug, Event, PartialEq, From)]
/// # enum RepositoryEvent {
/// #     FileV1(FileEventV1),
/// #     File(FileEvent),
/// #     Chat(ChatEvent),
/// # }
/// #
/// # // Actual Event we want to transform RepositoryEvent into
/// # #[derive(Clone, Copy, Debug, Event, From, PartialEq)]
/// # enum FileDomainEvent {
/// #     File(FileEvent),
/// # }
/// #
/// # #[derive(Clone, Copy, EventAdapter)]
/// # #[adapter(into = FileDomainEvent)]
/// # struct Adapter;
/// #
/// # impl adapter::Adapt<FileEvent> for Adapter {
/// #     type Strategy = strategy::AsIs;
/// # }
/// #
/// # impl adapter::Adapt<FileEventV1> for Adapter {
/// #     type Strategy = strategy::Into<FileEvent>;
/// # }
/// #
/// impl adapter::Adapt<ChatEvent> for Adapter {
///     type Strategy = strategy::Custom;
/// }
///
/// impl strategy::Customize<ChatEvent> for Adapter {
///     type Context = spell::Borrowed<
///         dyn Empty<Result<Self::Transformed, Self::Error>>
///     >;
///     type Error = Infallible;
///     type Transformed = FileDomainEvent;
///     type TransformedStream<'o> = stream::Empty<
///         Result<Self::Transformed, Self::Error>
///     >;
///
///     fn transform<'me: 'out, 'ctx: 'out, 'out>(
///         &'me self,
///         _event: ChatEvent,
///         context: &'ctx Self::Context
///     ) -> Self::TransformedStream<'out> {
///         context.stream()
///     }
///
/// }
///
/// pub struct EmptyProvider;
///
/// pub trait Empty<T> {
///     fn stream(&self) -> stream::Empty<T> {
///         stream::empty()
///     }
/// }
///
/// impl<T> Empty<T> for EmptyProvider {}
///
/// impl<T> Borrow<(dyn Empty<T> + 'static)> for EmptyProvider {
///     fn borrow(&self) -> &(dyn Empty<T> + 'static) {
///         self
///     }
/// }
///
/// #
/// # let assertion = async {
/// # let events = stream::iter::<[RepositoryEvent; 3]>([
/// #     FileEventV1.into(),
/// #     FileEvent.into(),
/// #     ChatEvent.into(),
/// # ]);
/// let transformed = Adapter
///     .transform_all(events, &EmptyProvider)
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
/// [`Borrow`]: std::borrow::Borrow
/// [`Custom`]: transformer::strategy::Custom
/// [`Error`]: Self::Error
/// [`Event`]: crate::es::Event
/// [`Skip`]: transformer::strategy::Skip
/// [`Split`]: transformer::strategy::Split
/// [`Strategies`]: Strategy
/// [`Strategy::Context`]: transformer::Strategy::Context
/// [`Transformed`]: Self::Transformed
/// [`Version`]: crate::es::event::Version
/// [`VersionedEvent`]: crate::es::VersionedEvent
pub trait Adapter<'ctx, Events, Ctx: ?Sized> {
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
                <Self as Adapter<'ctx, Events, Ctx>>::Transformed,
                <Self as Adapter<'ctx, Events, Ctx>>::Error,
            >,
        > + 'out
    where
        'ctx: 'out,
        Ctx: 'ctx,
        Events: 'out,
        Self: 'out;

    /// Converts all incoming [`Event`]s into [`Transformed`].
    ///
    /// [`Event`]: crate::es::Event
    /// [`Transformed`]: Self::Transformed
    fn transform_all<'me: 'out, 'out>(
        &'me self,
        events: Events,
        context: &'ctx Ctx,
    ) -> Self::TransformedStream<'out>;
}

impl<'ctx, A, Events, Ctx> Adapter<'ctx, Events, Ctx> for A
where
    A: Returning,
    Ctx: ?Sized + 'ctx,
    Events: Stream,
    Adapted<A>: Transformer<'ctx, Events::Item, Context<Ctx>>,
    A::Transformed:
        From<
            <Adapted<A> as TransformerTypes<
                'ctx,
                Events::Item,
                Context<Ctx>,
            >>::Transformed,
        >,
    A::Error:
        From<
            <Adapted<A> as TransformerTypes<
                'ctx,
                Events::Item,
                Context<Ctx>,
            >>::Error,
        >,
{
    type Error = <A as Returning>::Error;
    type Transformed = <A as Returning>::Transformed;
    type TransformedStream<'out>
    where
        'ctx: 'out,
        Ctx: 'ctx,
        Events: 'out,
        Self: 'out,
    = TransformedStream<'ctx, 'out, Adapted<A>, Events, Context<Ctx>>;

    fn transform_all<'me: 'out, 'out>(
        &'me self,
        events: Events,
        context: &'ctx Ctx,
    ) -> Self::TransformedStream<'out> {
        TransformedStream::new(
            RefCast::ref_cast(self),
            events,
            RefCast::ref_cast(context),
        )
    }
}

/// Wrapper around `context` in [`Adapter::transform_all()`] method used in pair
/// with [`spell::Borrowed`] to hack around orphan rules. Shouldn't be used
/// manually.
#[derive(Clone, Copy, Debug, RefCast)]
#[repr(transparent)]
pub struct Context<T: ?Sized>(pub T);

impl<T: ?Sized> Borrow<()> for Context<T> {
    fn borrow(&self) -> &() {
        &()
    }
}

impl<X: ?Sized, Y: ?Sized + Borrow<X>> Borrow<spell::Borrowed<X>>
    for Context<Y>
{
    fn borrow(&self) -> &spell::Borrowed<X> {
        RefCast::ref_cast(self.0.borrow())
    }
}

/// Wrapper type for [`Adapter`] to satisfy orphan rules on [`Event`] derive
/// macro. Shouldn't be used manually.
///
/// [`Event`]: crate::es::Event
#[derive(Debug, RefCast)]
#[repr(transparent)]
pub struct Adapted<A>(pub A);

impl<A> Returning for Adapted<A>
where
    A: Returning,
{
    type Error = A::Error;
    type Transformed = A::Transformed;
}

/// [`Stream`] for [`Adapter`] blanket impl.
///
/// Basically applies [`Transformer::transform()`] to every element of
/// [`Adapter`]s `Events` [`Stream`] and flattens it.
#[allow(explicit_outlives_requirements)] // false positive
#[pin_project]
pub struct TransformedStream<'ctx, 'out, Adapter, Events, Ctx>
where
    'ctx: 'out,
    Adapter: Transformer<'ctx, Events::Item, Ctx> + 'out,
    Ctx: ?Sized,
    Events: Stream,
{
    /// [`Stream`] of [`Event`]s to [`Transformer::transform()`].
    ///
    /// [`Event`]: crate::es::Event
    #[pin]
    events: Events,

    /// [`Transformer::transform()`] [`Stream`] to flatten.
    #[pin]
    transformed_stream:
        AdapterTransformedStream<'ctx, 'out, Events::Item, Adapter, Ctx>,

    /// [`Adapter`] implementor reference.
    adapter: &'out Adapter,

    /// [`Adapter`]'s `Context` reference.
    context: &'ctx Ctx,
}

impl<'ctx, 'out, Adapter, Events, Ctx> Debug
    for TransformedStream<'ctx, 'out, Adapter, Events, Ctx>
where
    Adapter: Debug + Transformer<'ctx, Events::Item, Ctx>,
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

/// Alias for [`TransformedStream::transformed_stream`].
type AdapterTransformedStream<'ctx, 'out, Event, Adapter, Ctx> = future::Either<
    <Adapter as TransformerTypes<'ctx, Event, Ctx>>::TransformedStream<'out>,
    stream::Empty<
        Result<
            <Adapter as TransformerTypes<'ctx, Event, Ctx>>::Transformed,
            <Adapter as TransformerTypes<'ctx, Event, Ctx>>::Error,
        >,
    >,
>;

impl<'ctx, 'out, Adapter, Events, Ctx>
    TransformedStream<'ctx, 'out, Adapter, Events, Ctx>
where
    Adapter: Transformer<'ctx, Events::Item, Ctx>,
    Ctx: ?Sized,
    Events: Stream,
{
    /// Creates a new [`TransformedStream`].
    fn new(adapter: &'out Adapter, events: Events, context: &'ctx Ctx) -> Self
    where
        'ctx: 'out,
    {
        Self {
            events,
            transformed_stream: stream::empty().right_stream(),
            adapter,
            context,
        }
    }
}

impl<'ctx, 'out, Adapter, Events, Ctx> Stream
    for TransformedStream<'ctx, 'out, Adapter, Events, Ctx>
where
    'ctx: 'out,
    Ctx: ?Sized,
    Adapter: Transformer<'ctx, Events::Item, Ctx> + Returning,
    Events: Stream,
    <Adapter as Returning>::Transformed: From<
        <Adapter as TransformerTypes<'ctx, Events::Item, Ctx>>::Transformed,
    >,
    <Adapter as Returning>::Error:
        From<<Adapter as TransformerTypes<'ctx, Events::Item, Ctx>>::Error>,
{
    type Item = Result<
        <Adapter as Returning>::Transformed,
        <Adapter as Returning>::Error,
    >;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut task::Context<'_>,
    ) -> task::Poll<Option<Self::Item>> {
        let mut this = self.project();

        loop {
            let flattened_res =
                futures::ready!(this.transformed_stream.as_mut().poll_next(cx));
            if let Some(ev) = flattened_res {
                return task::Poll::Ready(Some(
                    ev.map(Into::into).map_err(Into::into),
                ));
            }

            let outer_res = futures::ready!(this.events.as_mut().poll_next(cx));
            if let Some(event) = outer_res {
                let new_stream =
                    Adapter::transform(*this.adapter, event, *this.context);
                this.transformed_stream.set(new_stream.left_stream());
            } else {
                return task::Poll::Ready(None);
            }
        }
    }
}
