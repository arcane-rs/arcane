//! [`Strategy`] definition and default implementations.

use std::{
    convert::Infallible, fmt::Debug, iter::Iterator, marker::PhantomData,
};

use futures::{future, stream, Stream, StreamExt as _, TryStreamExt as _};

use crate::es::{adapter, event};

use super::{Transformer, WithStrategy};

/// Generalized [`Transformer`] for [`Versioned`] events.
///
/// [`Versioned`]: event::Versioned
pub trait Strategy<Adapter, Event, Ctx>
where
    Event: event::Versioned,
    Ctx: ?Sized,
{
    /// Error of this [`Strategy`].
    type Error;

    /// Converted [`Event`].
    ///
    /// [`Event`]: crate::es::Event
    type Transformed;

    /// [`Stream`] of [`Transformed`] [`Event`]s.
    ///
    /// [`Event`]: crate::es::Event
    /// [`Transformed`]: Self::Transformed
    #[rustfmt::skip]
    type TransformedStream<'out>:
        Stream<Item = Result<Self::Transformed, Self::Error>> + 'out;

    /// Converts incoming [`Event`] into [`Transformed`].
    ///
    /// [`Event`]: crate::es::Event
    /// [`Transformed`]: Self::Transformed
    fn transform<'me, 'ctx, 'out>(
        adapter: &'me Adapter,
        event: Event,
        context: &'ctx Ctx,
    ) -> Self::TransformedStream<'out>
    where
        'me: 'out,
        'ctx: 'out;
}

impl<Event, Adapter, Ctx> Transformer<Event, Ctx> for adapter::Wrapper<Adapter>
where
    Ctx: ?Sized,
    Event: event::Versioned,
    Adapter: WithStrategy<Event, Ctx> + adapter::WithError<Ctx>,
    Adapter::Strategy: Strategy<Adapter, Event, Ctx>,
    <Adapter as adapter::WithError<Ctx>>::Transformed:
        From<<Adapter::Strategy as Strategy<Adapter, Event, Ctx>>::Transformed>,
    <Adapter as adapter::WithError<Ctx>>::Error:
        From<<Adapter::Strategy as Strategy<Adapter, Event, Ctx>>::Error>,
{
    type Error = <Adapter::Strategy as Strategy<Adapter, Event, Ctx>>::Error;
    type Transformed =
        <Adapter::Strategy as Strategy<Adapter, Event, Ctx>>::Transformed;
    type TransformedStream<'out> = <Adapter::Strategy as Strategy<
        Adapter,
        Event,
        Ctx,
    >>::TransformedStream<'out>;

    fn transform<'me, 'ctx, 'out>(
        &'me self,
        event: Event,
        context: &'ctx Ctx,
    ) -> Self::TransformedStream<'out>
    where
        'me: 'out,
        'ctx: 'out,
    {
        <Adapter::Strategy as Strategy<Adapter, Event, Ctx>>::transform(
            &self.0, event, context,
        )
    }
}

/// [`Strategy`] for wrapping [`Event`]s in [`Initial`].
///
/// [`Event`]: crate::es::Event
/// [`Initial`]: event::Initial
#[derive(Clone, Debug)]
pub struct Initialized<InnerStrategy = AsIs>(PhantomData<InnerStrategy>);

impl<Adapter, Event, InnerStrategy, Ctx> Strategy<Adapter, Event, Ctx>
    for Initialized<InnerStrategy>
where
    Ctx: ?Sized,
    Event: event::Versioned,
    InnerStrategy: Strategy<Adapter, Event, Ctx>,
    InnerStrategy::Transformed: 'static,
    InnerStrategy::Error: 'static,
{
    type Error = InnerStrategy::Error;
    type Transformed = event::Initial<InnerStrategy::Transformed>;
    type TransformedStream<'out> = stream::MapOk<
        InnerStrategy::TransformedStream<'out>,
        WrapInitial<InnerStrategy::Transformed>,
    >;

    fn transform<'me, 'ctx, 'out>(
        adapter: &'me Adapter,
        event: Event,
        context: &'ctx Ctx,
    ) -> Self::TransformedStream<'out>
    where
        'me: 'out,
        'ctx: 'out,
    {
        InnerStrategy::transform(adapter, event, context).map_ok(event::Initial)
    }
}

type WrapInitial<Event> = fn(Event) -> event::Initial<Event>;

/// [`Strategy`] for skipping [`Event`]s.
///
/// Until [never] is stabilized, [`Adapter::Transformed`] must implement
/// [`From`] [`Unknown`].
///
/// [never]: https://doc.rust-lang.org/stable/std/primitive.never.html
/// [`Adapter::Transformed`]: crate::es::Adapter::Transformed
/// [`Event`]: crate::es::Event
#[derive(Clone, Copy, Debug)]
pub struct Skip;

impl<Adapter, Event, Ctx> Strategy<Adapter, Event, Ctx> for Skip
where
    Ctx: ?Sized,
    Event: event::Versioned,
    Adapter: adapter::WithError<Ctx>,
    Adapter::Transformed: 'static,
    Adapter::Error: 'static,
{
    type Error = Adapter::Error;
    type Transformed = Adapter::Transformed;
    type TransformedStream<'out> =
        stream::Empty<Result<Self::Transformed, Self::Error>>;

    fn transform<'me, 'ctx, 'out>(
        _: &'me Adapter,
        _: Event,
        _: &'ctx Ctx,
    ) -> Self::TransformedStream<'out>
    where
        'me: 'out,
        'ctx: 'out,
    {
        stream::empty()
    }
}

/// [`Strategy`] for passing [`Event`]s as is, without any conversions.
///
/// [`Event`]: crate::es::Event
#[derive(Clone, Copy, Debug)]
pub struct AsIs;

impl<Adapter, Event, Ctx> Strategy<Adapter, Event, Ctx> for AsIs
where
    Ctx: ?Sized,
    Event: event::Versioned + 'static,
{
    type Error = Infallible;
    type Transformed = Event;
    type TransformedStream<'out> =
        stream::Once<future::Ready<Result<Self::Transformed, Self::Error>>>;

    fn transform<'me, 'ctx, 'out>(
        _: &'me Adapter,
        event: Event,
        _: &'ctx Ctx,
    ) -> Self::TransformedStream<'out>
    where
        'me: 'out,
        'ctx: 'out,
    {
        stream::once(future::ready(Ok(event)))
    }
}

/// [`Strategy`] for converting [`Event`]s using [`From`] impl.
///
/// [`Event`]: crate::es::Event
#[derive(Copy, Clone, Debug)]
pub struct Into<I, InnerStrategy = AsIs>(PhantomData<(I, InnerStrategy)>);

impl<Adapter, Event, IntoEvent, InnerStrategy, Ctx>
    Strategy<Adapter, Event, Ctx> for Into<IntoEvent, InnerStrategy>
where
    Ctx: ?Sized,
    Event: event::Versioned,
    InnerStrategy: Strategy<Adapter, Event, Ctx>,
    InnerStrategy::Transformed: 'static,
    InnerStrategy::Error: 'static,
    IntoEvent: From<InnerStrategy::Transformed> + 'static,
{
    type Error = InnerStrategy::Error;
    type Transformed = IntoEvent;
    type TransformedStream<'out> = stream::MapOk<
        InnerStrategy::TransformedStream<'out>,
        IntoFn<InnerStrategy::Transformed, IntoEvent>,
    >;

    fn transform<'me, 'ctx, 'out>(
        adapter: &'me Adapter,
        event: Event,
        ctx: &'ctx Ctx,
    ) -> Self::TransformedStream<'out>
    where
        'me: 'out,
        'ctx: 'out,
    {
        InnerStrategy::transform(adapter, event, ctx).map_ok(IntoEvent::from)
    }
}

type IntoFn<FromEvent, IntoEvent> = fn(FromEvent) -> IntoEvent;

/// [`Strategy`] for splitting single [`Event`] into multiple. Implement
/// [`Splitter`] to define splitting logic.
///
/// [`Event`]: crate::es::Event
#[derive(Clone, Copy, Debug)]
pub struct Split<Into>(PhantomData<Into>);

/// Split single [`Event`] into multiple for [`Split`] [`Strategy`].
///
/// [`Event`]: crate::es::Event
pub trait Splitter<From, Into> {
    /// [`Iterator`] of split [`Event`]s.
    ///
    /// [`Event`]: crate::es::Event
    type Iterator: Iterator<Item = Into>;

    /// Splits [`Event`].
    ///
    /// [`Event`]: crate::es::Event
    fn split(&self, event: From) -> Self::Iterator;
}

impl<Adapter, Event, IntoEvent, Ctx> Strategy<Adapter, Event, Ctx>
    for Split<IntoEvent>
where
    Ctx: ?Sized,
    Event: event::Versioned,
    IntoEvent: 'static,
    Adapter: Splitter<Event, IntoEvent>,
    Adapter::Iterator: 'static,
{
    type Error = Infallible;
    type Transformed = <Adapter::Iterator as Iterator>::Item;
    type TransformedStream<'out> = SplitStream<Adapter, Event, IntoEvent>;

    fn transform<'me, 'ctx, 'out>(
        adapter: &'me Adapter,
        event: Event,
        _: &'ctx Ctx,
    ) -> Self::TransformedStream<'out>
    where
        'me: 'out,
        'ctx: 'out,
    {
        stream::iter(adapter.split(event)).map(Ok)
    }
}

type SplitStream<Adapter, From, Into> = stream::Map<
    stream::Iter<<Adapter as Splitter<From, Into>>::Iterator>,
    fn(
        <<Adapter as Splitter<From, Into>>::Iterator as Iterator>::Item,
    ) -> Result<
        <<Adapter as Splitter<From, Into>>::Iterator as Iterator>::Item,
        Infallible,
    >,
>;
