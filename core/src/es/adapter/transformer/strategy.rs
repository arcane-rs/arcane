//! [`Strategy`] definition and default implementations.

use std::{
    any::Any,
    convert::Infallible,
    fmt::{Debug, Formatter},
    iter::Iterator,
    marker::PhantomData,
};

use futures::{future, stream, Stream, StreamExt as _, TryStreamExt as _};

use crate::es::event;

use super::{Transformer, WithStrategy};

/// Generalized [`Transformer`].
pub trait Strategy<Adapter, Event> {
    /// Context for converting [`Event`]s.
    ///
    /// [`Event`]: crate::es::Event
    type Context: ?Sized;

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
        context: &'ctx Self::Context,
    ) -> Self::TransformedStream<'out>
    where
        'me: 'out,
        'ctx: 'out;
}

impl<Event, Adapter> Transformer<Event> for Adapter
where
    Adapter: WithStrategy<Event>,
    Adapter::Strategy: Strategy<Adapter, Event>,
{
    type Context = <Adapter::Strategy as Strategy<Adapter, Event>>::Context;
    type Error = <Adapter::Strategy as Strategy<Adapter, Event>>::Error;
    type Transformed =
        <Adapter::Strategy as Strategy<Adapter, Event>>::Transformed;
    type TransformedStream<'out> = <Adapter::Strategy as Strategy<
        Adapter,
        Event,
    >>::TransformedStream<'out>;

    fn transform<'me, 'ctx, 'out>(
        &'me self,
        event: Event,
        context: &'ctx Self::Context,
    ) -> Self::TransformedStream<'out>
    where
        'me: 'out,
        'ctx: 'out,
    {
        <Adapter::Strategy as Strategy<Adapter, Event>>::transform(
            self, event, context,
        )
    }
}

/// [`Strategy`] for wrapping [`Event`]s in [`Initial`].
///
/// [`Event`]: crate::es::Event
/// [`Initial`]: event::Initial
#[derive(Clone, Debug)]
pub struct Initialized<S>(PhantomData<S>);

impl<Adapter, Event, InnerStrategy> Strategy<Adapter, Event>
    for Initialized<InnerStrategy>
where
    InnerStrategy: Strategy<Adapter, Event>,
    InnerStrategy::Transformed: 'static,
{
    type Context = InnerStrategy::Context;
    type Error = InnerStrategy::Error;
    type Transformed = event::Initial<InnerStrategy::Transformed>;
    type TransformedStream<'out> = stream::MapOk<
        InnerStrategy::TransformedStream<'out>,
        WrapInitial<InnerStrategy::Transformed>,
    >;

    fn transform<'me, 'ctx, 'out>(
        adapter: &'me Adapter,
        event: Event,
        context: &'ctx Self::Context,
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

/// As [`Skip`] [`Strategy`] isn't parametrised by [`Strategy::Transformed`]
/// [`Event`], this type expresses 'never going to be constructed'.
///
/// [`Event`]: crate::es::Event
// TODO: replace with `never`(`!`), once it's stabilized.
#[derive(Clone, Copy, Debug)]
pub enum Unknown {}

impl<Adapter, Event> Strategy<Adapter, Event> for Skip {
    type Context = dyn Any;
    type Error = Infallible;
    type Transformed = Unknown;
    type TransformedStream<'out> = stream::Empty<Result<Unknown, Infallible>>;

    fn transform<'me, 'ctx, 'out>(
        _: &'me Adapter,
        _: Event,
        _: &'ctx Self::Context,
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

impl<Adapter, Event: 'static> Strategy<Adapter, Event> for AsIs {
    type Context = dyn Any;
    type Error = Infallible;
    type Transformed = Event;
    type TransformedStream<'out> =
        stream::Once<future::Ready<Result<Event, Self::Error>>>;

    fn transform<'me, 'ctx, 'out>(
        _: &'me Adapter,
        event: Event,
        _: &'ctx Self::Context,
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
pub struct Into<Into>(PhantomData<Into>);

impl<Event> Clone for Into<Event> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}

impl<Event> Copy for Into<Event> {}

impl<Event> Debug for Into<Event> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Into").finish()
    }
}

impl<Adapter, Event, IntoEvent> Strategy<Adapter, Event> for Into<IntoEvent>
where
    IntoEvent: From<Event> + 'static,
{
    type Context = dyn Any;
    type Error = Infallible;
    type Transformed = IntoEvent;
    type TransformedStream<'out> =
        stream::Once<future::Ready<Result<IntoEvent, Infallible>>>;

    fn transform<'me, 'ctx, 'out>(
        _: &'me Adapter,
        event: Event,
        _: &'ctx Self::Context,
    ) -> Self::TransformedStream<'out>
    where
        'me: 'out,
        'ctx: 'out,
    {
        stream::once(future::ready(Ok(IntoEvent::from(event))))
    }
}

/// [`Strategy`] for splitting single [`Event`] into multiple. Implement
/// [`Splitter`] to define splitting logic.
///
/// [`Event`]: crate::es::Event
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

impl<Event> Clone for Split<Event> {
    fn clone(&self) -> Self {
        Self(PhantomData)
    }
}

impl<Event> Copy for Split<Event> {}

impl<Event> Debug for Split<Event> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Into").finish()
    }
}

impl<Adapter, Event, IntoEvent> Strategy<Adapter, Event> for Split<IntoEvent>
where
    IntoEvent: 'static,
    Adapter: Splitter<Event, IntoEvent>,
    Adapter::Iterator: 'static,
{
    type Context = dyn Any;
    type Error = Infallible;
    type Transformed = <Adapter::Iterator as Iterator>::Item;
    type TransformedStream<'out> = SplitStream<Adapter, Event, IntoEvent>;

    fn transform<'me, 'ctx, 'out>(
        adapter: &'me Adapter,
        event: Event,
        _: &'ctx Self::Context,
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
