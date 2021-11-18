//! [`Strategy`] definition and default implementations.

pub mod as_is;
pub mod custom;
pub mod into;
pub mod skip;
pub mod split;

use std::borrow::Borrow;

use futures::Stream;
use ref_cast::RefCast;

use crate::es::{event, event::adapter};

use super::{Adapt, Transformer};

#[doc(inline)]
pub use self::{
    as_is::AsIs,
    custom::{Custom, Customize},
    into::Into,
    skip::Skip,
    split::{Split, Splitter},
};

/// Generalized [`Transformer`] for [`Versioned`] events.
///
/// [`Versioned`]: event::Versioned
pub trait Strategy<Adapter, Event> {
    /// Context of this [`Strategy`].
    ///
    /// In real world this is usually `dyn Trait`. In that case,
    /// [`Adapter::transform_all()`][1] will expect type which can be
    /// [`Borrow`]ed as `dyn Trait`.
    ///
    /// [1]: adapter::Adapter::transform_all
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
    type TransformedStream<'out>: Stream<
            Item = Result<
                <Self as Strategy<Adapter, Event>>::Transformed,
                <Self as Strategy<Adapter, Event>>::Error,
            >,
        > + 'out;

    /// Converts incoming [`Event`] into [`Transformed`].
    ///
    /// [`Event`]: crate::es::Event
    /// [`Transformed`]: Self::Transformed
    fn transform<'me: 'out, 'ctx: 'out, 'out>(
        adapter: &'me Adapter,
        event: Event,
        context: &'ctx Self::Context,
    ) -> Self::TransformedStream<'out>;
}

impl<'ctx, Event, Adapter, Ctx> Transformer<'ctx, Event, Ctx>
    for adapter::Wrapper<Adapter>
where
    Event: event::VersionedOrRaw,
    Adapter: Adapt<Event> + adapter::Returning,
    Adapter::Strategy: Strategy<Adapter, Event>,
    Adapter::Transformed:
        From<<Adapter::Strategy as Strategy<Adapter, Event>>::Transformed>,
    Adapter::Error:
        From<<Adapter::Strategy as Strategy<Adapter, Event>>::Error>,
    Ctx: Borrow<<Adapter::Strategy as Strategy<Adapter, Event>>::Context>
        + ?Sized,
    <Adapter::Strategy as Strategy<Adapter, Event>>::Context: 'ctx,
{
    type Error = <Adapter::Strategy as Strategy<Adapter, Event>>::Error;
    type Transformed =
        <Adapter::Strategy as Strategy<Adapter, Event>>::Transformed;
    type TransformedStream<'out> = <Adapter::Strategy as Strategy<
        Adapter,
        Event,
    >>::TransformedStream<'out>;

    fn transform<'me, 'out>(
        &'me self,
        event: Event,
        context: &'ctx Ctx,
    ) -> Self::TransformedStream<'out>
    where
        'me: 'out,
        'ctx: 'out,
    {
        <Adapter::Strategy as Strategy<Adapter, Event>>::transform(
            &self.0,
            event,
            context.borrow(),
        )
    }
}

/// TODO
#[derive(Clone, Copy, Debug, RefCast)]
#[repr(transparent)]
pub struct Context<T: ?Sized>(pub T);

/// TODO
#[derive(Clone, Copy, Debug, RefCast)]
#[repr(transparent)]
pub struct InnerContext<T: ?Sized>(pub T);

impl<T: ?Sized> Borrow<()> for Context<T> {
    fn borrow(&self) -> &() {
        &()
    }
}

impl<X: ?Sized, Y: ?Sized + Borrow<X>> Borrow<InnerContext<X>> for Context<Y> {
    fn borrow(&self) -> &InnerContext<X> {
        RefCast::ref_cast(self.0.borrow())
    }
}
