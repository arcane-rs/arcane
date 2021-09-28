//! [`Split`] [`Strategy`] definition.

use std::marker::PhantomData;

use futures::{stream, StreamExt as _};

use crate::es::{adapter, event};

use super::Strategy;

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
    Adapter: Splitter<Event, IntoEvent> + adapter::Returning,
    Adapter::Iterator: 'static,
    Adapter::Error: 'static,
    Ctx: ?Sized,
    Event: event::VersionedOrRaw,
    IntoEvent: 'static,
{
    type Error = Adapter::Error;
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
        <Adapter as adapter::Returning>::Error,
    >,
>;
