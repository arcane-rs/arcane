//! [`Strategy`] definition and default implementations.

use std::{
    convert::Infallible, fmt::Debug, iter::Iterator, marker::PhantomData,
};

use futures::{future, stream, Stream, StreamExt as _, TryStreamExt as _};

use crate::es::{
    adapter::{self, Correct},
    event,
};

use super::{Transformer, WithStrategy};

/// Generalized [`Transformer`] for [`Versioned`] events.
///
/// [`Versioned`]: event::Versioned
pub trait Strategy<Adapter, Event>
where
    Event: event::Versioned,
{
    /// TODO
    type Context<Impl>: Correct;

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
    type TransformedStream<'out, Ctx: 'out>: Stream<
            Item = Result<
                <Self as Strategy<Adapter, Event>>::Transformed,
                <Self as Strategy<Adapter, Event>>::Error,
            >,
        > + 'out;

    /// Converts incoming [`Event`] into [`Transformed`].
    ///
    /// [`Event`]: crate::es::Event
    /// [`Transformed`]: Self::Transformed
    fn transform<'me, 'ctx, 'out, Ctx>(
        adapter: &'me Adapter,
        event: Event,
        context: &'ctx Ctx,
    ) -> Self::TransformedStream<'out, Ctx>
    where
        'me: 'out,
        'ctx: 'out,
        Ctx: 'out;
}

impl<Event, Adapter> Transformer<Event> for adapter::Wrapper<Adapter>
where
    Event: event::Versioned,
    Adapter: WithStrategy<Event>,
    Adapter::Strategy: Strategy<Adapter, Event>,
{
    type Context<Impl> =
        <Adapter::Strategy as Strategy<Adapter, Event>>::Context<Impl>;

    type Error = <Adapter::Strategy as Strategy<Adapter, Event>>::Error;

    type Transformed =
        <Adapter::Strategy as Strategy<Adapter, Event>>::Transformed;

    type TransformedStream<'out, Ctx: 'out> = <Adapter::Strategy as Strategy<
        Adapter,
        Event,
    >>::TransformedStream<'out, Ctx>;

    fn transform<'me, 'ctx, 'out, Ctx>(
        &'me self,
        event: Event,
        context: &'ctx Ctx,
    ) -> Self::TransformedStream<'out, Ctx>
    where
        'me: 'out,
        'ctx: 'out,
        Ctx: 'out,
    {
        <Adapter::Strategy as Strategy<Adapter, Event>>::transform(
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

impl<Adapter, Event, InnerStrategy> Strategy<Adapter, Event>
    for Initialized<InnerStrategy>
where
    Event: event::Versioned,
    InnerStrategy: Strategy<Adapter, Event>,
    InnerStrategy::Transformed: 'static,
    InnerStrategy::Error: 'static,
{
    type Context<Impl> = InnerStrategy::Context<Impl>;

    type Error = InnerStrategy::Error;

    type Transformed = event::Initial<InnerStrategy::Transformed>;

    type TransformedStream<'out, Ctx: 'out> = stream::MapOk<
        InnerStrategy::TransformedStream<'out, Ctx>,
        WrapInitial<InnerStrategy::Transformed>,
    >;

    fn transform<'me, 'ctx, 'out, Ctx>(
        adapter: &'me Adapter,
        event: Event,
        context: &'ctx Ctx,
    ) -> Self::TransformedStream<'out, Ctx>
    where
        'me: 'out,
        'ctx: 'out,
        Ctx: 'out,
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

impl<Adapter, Event> Strategy<Adapter, Event> for Skip
where
    Event: event::Versioned,
    Adapter: adapter::WithError,
    Adapter::Transformed: 'static,
    Adapter::Error: 'static,
{
    type Context<Impl> = Any<Impl>;
    type Error = Adapter::Error;
    type Transformed = Adapter::Transformed;
    type TransformedStream<'out, Ctx: 'out> =
        stream::Empty<Result<Self::Transformed, Self::Error>>;

    fn transform<'me, 'ctx, 'out, Ctx>(
        _: &'me Adapter,
        _: Event,
        _: &'ctx Ctx,
    ) -> Self::TransformedStream<'out, Ctx>
    where
        'me: 'out,
        'ctx: 'out,
        Ctx: 'out,
    {
        stream::empty()
    }
}

/// [`Strategy`] for passing [`Event`]s as is, without any conversions.
///
/// [`Event`]: crate::es::Event
#[derive(Clone, Copy, Debug)]
pub struct AsIs;

impl<Adapter, Event> Strategy<Adapter, Event> for AsIs
where
    Event: event::Versioned + 'static,
{
    type Context<Impl> = Any<Impl>;
    type Error = Infallible;
    type Transformed = Event;
    type TransformedStream<'out, Ctx: 'out> =
        stream::Once<future::Ready<Result<Self::Transformed, Self::Error>>>;

    fn transform<'me, 'ctx, 'out, Ctx>(
        _: &'me Adapter,
        event: Event,
        _: &'ctx Ctx,
    ) -> Self::TransformedStream<'out, Ctx>
    where
        'me: 'out,
        'ctx: 'out,
        Ctx: 'out,
    {
        stream::once(future::ready(Ok(event)))
    }
}

/// [`Strategy`] for converting [`Event`]s using [`From`] impl.
///
/// [`Event`]: crate::es::Event
#[derive(Copy, Clone, Debug)]
pub struct Into<I, InnerStrategy = AsIs>(PhantomData<(I, InnerStrategy)>);

impl<Adapter, Event, IntoEvent, InnerStrategy> Strategy<Adapter, Event>
    for Into<IntoEvent, InnerStrategy>
where
    Event: event::Versioned,
    InnerStrategy: Strategy<Adapter, Event>,
    InnerStrategy::Transformed: 'static,
    InnerStrategy::Error: 'static,
    IntoEvent: From<InnerStrategy::Transformed> + 'static,
{
    type Context<Impl> = InnerStrategy::Context<Impl>;
    type Error = InnerStrategy::Error;
    type Transformed = IntoEvent;
    type TransformedStream<'out, Ctx: 'out> = stream::MapOk<
        InnerStrategy::TransformedStream<'out, Ctx>,
        IntoFn<InnerStrategy::Transformed, IntoEvent>,
    >;

    fn transform<'me, 'ctx, 'out, Ctx>(
        adapter: &'me Adapter,
        event: Event,
        ctx: &'ctx Ctx,
    ) -> Self::TransformedStream<'out, Ctx>
    where
        'me: 'out,
        'ctx: 'out,
        Ctx: 'out,
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

impl<Adapter, Event, IntoEvent> Strategy<Adapter, Event> for Split<IntoEvent>
where
    Event: event::Versioned,
    IntoEvent: 'static,
    Adapter: Splitter<Event, IntoEvent>,
    Adapter::Iterator: 'static,
{
    type Context<Impl> = Any<Impl>;
    type Error = Infallible;
    type Transformed = <Adapter::Iterator as Iterator>::Item;
    type TransformedStream<'out, Ctx: 'out> =
        SplitStream<Adapter, Event, IntoEvent>;

    fn transform<'me, 'ctx, 'out, Ctx>(
        adapter: &'me Adapter,
        event: Event,
        _: &'ctx Ctx,
    ) -> Self::TransformedStream<'out, Ctx>
    where
        'me: 'out,
        'ctx: 'out,
        Ctx: 'out,
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

/// TODO
#[derive(Debug)]
pub struct Any<T>(T);

impl<T> Correct for Any<T> {}

/// TODO
#[derive(Debug)]
pub struct And<L, R>(L, R);

impl<L, R> Correct for And<L, R>
where
    L: Correct,
    R: Correct,
{
}
