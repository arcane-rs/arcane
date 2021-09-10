//! [`Transformer`] definitions.

pub mod strategy;

use futures::Stream;

#[doc(inline)]
pub use strategy::Strategy;

/// Facility to convert [`Event`]s.
/// Typical use cases include (but are not limited to):
///
/// - [`Skip`]ping unused [`Event`]s;
/// - Transforming (ex: from one [`Version`] to another);
/// - [`Split`]ting existing [`Event`]s into more granular ones.
///
/// To reduce boilerplate consider using [`WithStrategy`] with some [`Strategy`]
/// instead of implementing this trait manually.
///
/// [`Event`]: crate::es::Event
/// [`Skip`]: strategy::Skip
/// [`Split`]: strategy::Split
/// [`Version`]: crate::es::event::Version
pub trait Transformer<Event> {
    /// Context for converting [`Event`]s.
    ///
    /// [`Event`]: crate::es::Event
    type Context: ?Sized;

    /// Error of this [`Transformer`].
    type Error;

    /// Converted [`Event`].
    ///
    /// [`Event`]: crate::es::Event
    type Transformed;

    /// [`Stream`] of [`Transformed`] [`Event`]s.
    ///
    /// [`Event`]: crate::es::Event
    /// [`Transformed`]: Self::Transformed
    type TransformedStream<'me, 'ctx>: Stream<
        Item = Result<Self::Transformed, Self::Error>,
    >;

    /// Converts incoming [`Event`] into [`Transformed`].
    ///
    /// [`Event`]: crate::es::Event
    /// [`Transformed`]: Self::Transformed
    fn transform<'me, 'ctx>(
        &'me self,
        event: Event,
        context: &'ctx Self::Context,
    ) -> Self::TransformedStream<'me, 'ctx>;
}

/// Instead of implementing [`Transformer`] manually, you can use this trait
/// with some [`Strategy`].
pub trait WithStrategy<Event>: Sized {
    /// [`Strategy`] to transform [`Event`] with.
    ///
    /// [`Event`]: crate::es::Event
    type Strategy: Strategy<Self, Event>;
}

/// TODO
pub trait TransformedBy<Adapter> {}

impl<Ev, A> TransformedBy<A> for Ev where A: Transformer<Ev> {}

pub mod specialization {
    //! TODO

    #![allow(clippy::unused_self)]

    use std::marker::PhantomData;

    use crate::es::{
        adapter::{
            transformer::{strategy, WithStrategy},
            Transformer,
        },
        event,
    };
    use futures::{future, stream, StreamExt as _};

    /// TODO
    pub trait Get<const N: usize> {
        /// TODO
        type Out;

        /// TODO
        fn get(&self) -> Option<&Self::Out>;

        /// TODO
        fn unwrap(self) -> Self::Out;
    }

    /// TODO
    pub trait EnumSize {
        /// TODO
        const SIZE: usize;
    }

    /// TODO
    #[derive(Debug)]
    pub struct Wrap<Adapter, Event, TransformedEvent>(
        /// TODO
        pub Adapter,
        /// TODO
        pub Event,
        /// TODO
        pub PhantomData<TransformedEvent>,
    );

    // With Skip Adapter

    /// TODO
    pub trait TransformedBySkipAdapter {
        /// TODO
        fn get_tag(&self) -> AdapterSkippedTag;
    }

    impl<Adapter, Event, TransformedEvent> TransformedBySkipAdapter
        for &&&&Wrap<&Adapter, &Event, TransformedEvent>
    where
        Adapter: WithStrategy<Event, Strategy = strategy::Skip>,
    {
        fn get_tag(&self) -> AdapterSkippedTag {
            AdapterSkippedTag
        }
    }

    /// TODO
    #[derive(Clone, Copy, Debug)]
    pub struct AdapterSkippedTag;

    impl AdapterSkippedTag {
        /// TODO
        pub fn transform_event<Adapter, Event, TrEvent, Ctx, Err>(
            self,
            _: &Adapter,
            _: Event,
            _: &Ctx,
        ) -> stream::Empty<Result<TrEvent, Err>>
        where
            Ctx: ?Sized,
        {
            stream::empty()
        }
    }

    // With Adapter

    /// TODO
    pub trait TransformedByAdapter {
        /// TODO
        fn get_tag(&self) -> AdapterTag;
    }

    impl<Adapter, Event, TransformedEvent> TransformedByAdapter
        for &&&Wrap<&Adapter, &Event, TransformedEvent>
    where
        Adapter: Transformer<Event>,
    {
        fn get_tag(&self) -> AdapterTag {
            AdapterTag
        }
    }

    /// TODO
    #[derive(Clone, Copy, Debug)]
    pub struct AdapterTag;

    impl AdapterTag {
        /// TODO
        pub fn transform_event<'me, 'ctx, Adapter, Event, TrEvent, Ctx, Err>(
            self,
            adapter: &'me Adapter,
            ev: Event,
            context: &'ctx Ctx,
        ) -> AdapterTagStream<'me, 'ctx, Adapter, Event, TrEvent, Err>
        where
            Event: 'static,
            Ctx: ?Sized,
            Adapter: Transformer<Event, Context = Ctx>,
            TrEvent: From<Adapter::Transformed>,
            Err: From<Adapter::Error>,
        {
            <Adapter as Transformer<Event>>::transform(adapter, ev, context)
                .map(|res| res.map(Into::into).map_err(Into::into))
        }
    }

    type AdapterTagStream<'me, 'ctx, Adapter, Event, TrEvent, Err> =
        stream::Map<
            <Adapter as Transformer<Event>>::TransformedStream<'me, 'ctx>,
            fn(
                Result<
                    <Adapter as Transformer<Event>>::Transformed,
                    <Adapter as Transformer<Event>>::Error,
                >,
            ) -> Result<TrEvent, Err>,
        >;

    // With From

    /// TODO
    pub trait TransformedByFrom {
        /// TODO
        fn get_tag(&self) -> FromTag;
    }

    impl<Adapter, Event, TransformedEvent> TransformedByFrom
        for &&Wrap<&Adapter, &Event, TransformedEvent>
    where
        TransformedEvent: From<Event>,
    {
        fn get_tag(&self) -> FromTag {
            FromTag
        }
    }

    /// TODO
    #[derive(Clone, Copy, Debug)]
    pub struct FromTag;

    impl FromTag {
        /// TODO
        pub fn transform_event<Adapter, Event, TrEvent, Ctx, Err>(
            self,
            _: &Adapter,
            ev: Event,
            _: &Ctx,
        ) -> stream::Once<future::Ready<Result<TrEvent, Err>>>
        where
            Ctx: ?Sized,
            TrEvent: From<Event>,
        {
            stream::once(future::ready(Ok(ev.into())))
        }
    }

    // With From Initial

    /// TODO
    pub trait TransformedByFromInitial {
        /// TODO
        fn get_tag(&self) -> FromInitialTag;
    }

    impl<Adapter, Event, TransformedEvent> TransformedByFromInitial
        for &Wrap<&Adapter, &Event, TransformedEvent>
    where
        TransformedEvent: From<event::Initial<Event>>,
    {
        fn get_tag(&self) -> FromInitialTag {
            FromInitialTag
        }
    }

    /// TODO
    #[derive(Clone, Copy, Debug)]
    pub struct FromInitialTag;

    impl FromInitialTag {
        /// TODO
        pub fn transform_event<Adapter, Event, TrEvent, Ctx, Err>(
            self,
            _: &Adapter,
            ev: Event,
            _: &Ctx,
        ) -> stream::Once<future::Ready<Result<TrEvent, Err>>>
        where
            Ctx: ?Sized,
            TrEvent: From<event::Initial<Event>>,
        {
            stream::once(future::ready(Ok(event::Initial(ev).into())))
        }
    }

    // Skip

    /// TODO
    pub trait TransformedByEmpty {
        /// TODO
        fn get_tag(&self) -> EmptyTag;
    }

    impl<Adapter, Event, TransformedEvent> TransformedByEmpty
        for Wrap<&Adapter, &Event, TransformedEvent>
    {
        fn get_tag(&self) -> EmptyTag {
            EmptyTag
        }
    }

    /// TODO
    #[derive(Clone, Copy, Debug)]
    pub struct EmptyTag;

    impl EmptyTag {
        /// TODO
        pub fn transform_event<Adapter, Event, TrEvent, Ctx, Err>(
            self,
            _: &Adapter,
            _: Event,
            _: &Ctx,
        ) -> stream::Empty<Result<TrEvent, Err>>
        where
            Ctx: ?Sized,
        {
            stream::empty()
        }
    }
}
