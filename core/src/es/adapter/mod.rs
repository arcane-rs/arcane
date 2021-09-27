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
pub use self::transformer::Transformer;

/// TODO
pub trait WithError {
    /// TODO
    type Error;

    /// TODO
    type Transformed;
}

/// TODO
#[derive(Debug, RefCast)]
#[repr(transparent)]
pub struct Wrapper<A>(pub A);

impl<A> WithError for Wrapper<A>
where
    A: WithError,
{
    type Error = A::Error;
    type Transformed = A::Transformed;
}

/// Facility to convert [`Event`]s.
/// Typical use cases include (but are not limited to):
///
/// - [`Skip`]ping unused [`Event`]s;
/// - Transforming (ex: from one [`Version`] to another);
/// - [`Split`]ting existing [`Event`]s into more granular ones.
///
/// Provided with blanket impl for [`Transformer`] implementors, so usually you
/// shouldn't implement it manually.
///
/// [`Event`]: crate::es::Event
/// [`Skip`]: transformer::strategy::Skip
/// [`Split`]: transformer::strategy::Split
/// [`Version`]: crate::es::event::Version
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
        Events: 'out;

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
    Events: Stream,
    Ctx: 'static + ?Sized,
    A: WithError,
    Wrapper<A>: Transformer<Events::Item, Ctx> + 'static,
    <A as WithError>::Transformed:
        From<<Wrapper<A> as Transformer<Events::Item, Ctx>>::Transformed>,
    <A as WithError>::Error:
        From<<Wrapper<A> as Transformer<Events::Item, Ctx>>::Error>,
{
    type Error = <A as WithError>::Error;
    type Transformed = <A as WithError>::Transformed;
    type TransformedStream<'out>
    where
        Events: 'out,
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

#[pin_project]
/// [`Stream`] for [`Adapter`] blanket impl.
pub struct TransformedStream<'out, Adapter, Events, Ctx>
where
    Events: Stream,
    Adapter: Transformer<Events::Item, Ctx>,
    Ctx: ?Sized,
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
    Events: Debug + Stream,
    Adapter: Debug + Transformer<Events::Item, Ctx>,
    Ctx: Debug + ?Sized,
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
    Events: Stream,
    Adapter: Transformer<Events::Item, Ctx>,
    Ctx: ?Sized,
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
    Events: Stream,
    Ctx: ?Sized,
    Adapter: Transformer<Events::Item, Ctx> + WithError,
    <Adapter as WithError>::Transformed:
        From<<Adapter as Transformer<Events::Item, Ctx>>::Transformed>,
    <Adapter as WithError>::Error:
        From<<Adapter as Transformer<Events::Item, Ctx>>::Error>,
{
    type Item = Result<
        <Adapter as WithError>::Transformed,
        <Adapter as WithError>::Error,
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

#[cfg(feature = "codegen")]
pub mod codegen {
    //! Re-exports for [`Transformer`] derive macro.
    //!
    //! [`Transformer`]: crate::es::adapter::Transformer

    pub use futures;
}
