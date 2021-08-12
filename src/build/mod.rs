use async_trait::async_trait;
use derive_more::{Deref, DerefMut};
use std::{fmt, marker::PhantomData, thread::JoinHandle};

use crate::{cqrs, es};

#[derive(Clone, Copy, Debug)]
pub struct Nothing;

#[derive(Clone, Copy, Debug, Deref, DerefMut)]
pub struct Handler<T, Ctx: ?Sized = Nothing> {
    #[deref]
    #[deref_mut]
    handler: T,
    context: Ctx,
}

impl<T> Handler<T> {
    #[inline]
    #[must_use]
    pub fn new(handler: T) -> Self {
        Self {
            handler,
            context: Nothing,
        }
    }

    #[inline]
    #[must_use]
    pub fn with<Ctx>(self, context: Ctx) -> Handler<T, With<Ctx>> {
        Handler {
            handler: self.handler,
            context: With::data(context),
        }
    }
}

impl<T, Ctx, And> Handler<T, With<Ctx, And>> {
    #[inline]
    #[must_use]
    pub fn and<NewCtx>(
        self,
        context: NewCtx,
    ) -> Handler<T, With<Ctx, <With<NewCtx, And> as SinkType<NewCtx>>::Out>>
    where
        With<NewCtx, And>: SinkType<NewCtx>,
    {
        Handler {
            handler: self.handler,
            context: self.context.and(context),
        }
    }
}

impl<T, Ctx: ?Sized> Handler<T, Ctx> {
    #[inline]
    #[must_use]
    pub fn context(&self) -> &Ctx {
        &self.context
    }
}

#[async_trait(?Send)]
impl<Cmd, T> cqrs::CommandHandler<Cmd> for Handler<T>
where
    Cmd: cqrs::Command,
    T: cqrs::CommandHandler<Cmd>,
{
    type Result = <T as cqrs::CommandHandler<Cmd>>::Result;

    async fn handle(&mut self, cmd: Cmd) -> Self::Result
    where
        Cmd: 'async_trait,
    {
        self.handler.handle(cmd).await
    }
}

pub type Of<T, And = Nothing> = With<T, And>;
pub type And<T, And = Nothing> = With<T, And>;

#[derive(Clone, Copy, Debug, Deref, DerefMut)]
pub struct With<T, And: ?Sized = Nothing> {
    #[deref]
    #[deref_mut]
    data: T,
    more: And,
}

impl<T> With<T> {
    #[inline]
    #[must_use]
    pub fn data(data: T) -> Self {
        Self {
            data,
            more: Nothing,
        }
    }
}

impl<A, And> With<A, And> {
    #[inline]
    #[must_use]
    pub fn and<B>(self, data: B) -> With<A, <With<B, And> as SinkType<B>>::Out>
    where
        With<B, And>: SinkType<B>,
    {
        With { data, more: self }.sink_type()
    }

    #[inline]
    #[must_use]
    pub fn into_inner(self) -> (A, And) {
        (self.data, self.more)
    }
}

impl<A, And: ?Sized> With<A, And> {
    #[inline]
    #[must_use]
    pub fn this(&self) -> &A {
        &self.data
    }

    #[inline]
    #[must_use]
    pub fn this_mut(&mut self) -> &mut A {
        &mut self.data
    }

    #[inline]
    #[must_use]
    pub fn other(&self) -> &And {
        &self.more
    }

    #[inline]
    #[must_use]
    pub fn other_mut(&mut self) -> &mut And {
        &mut self.more
    }
}

impl<A> SinkType<A> for With<A, Nothing> {
    type Out = Self;

    #[inline]
    fn sink_type(self) -> Self::Out {
        self
    }
}

impl<A, B, And> SinkType<A> for With<A, With<B, And>>
where
    With<A, And>: SinkType<A>,
{
    type Out = With<B, <With<A, And> as SinkType<A>>::Out>;

    #[inline]
    fn sink_type(self) -> Self::Out {
        With {
            data: self.more.data,
            more: With {
                data: self.data,
                more: self.more.more,
            }
            .sink_type(),
        }
    }
}

pub trait SinkType<T> {
    type Out;

    #[must_use]
    fn sink_type(self) -> Self::Out;
}

pub struct New<T: ?Sized>(PhantomData<JoinHandle<Box<T>>>);

impl<T: ?Sized> New<T> {
    #[inline]
    #[must_use]
    pub fn aggregate() -> Self {
        Self(PhantomData)
    }
}

impl<T: ?Sized> Clone for New<T> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<T: ?Sized> Copy for New<T> {}

impl<T: ?Sized> fmt::Debug for New<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("New").field(&self.0).finish()
    }
}

/*

pub struct Command<Handler: ?Sized, Args: ?Sized> {
    _handler: PhantomData<JoinHandle<Box<Handler>>>,
    args: Args,
}

impl<Handler: ?Sized, Args> Command<Handler, With<Args>> {
    /// Creates a new [`Command`] to the given `Handler` with the given `args`.
    #[inline]
    #[must_use]
    pub fn with<A>(args: A) -> Self
    where
        Args: From<A>,
    {
        Self {
            _handler: PhantomData,
            args: With(args.into()),
        }
    }
}

impl<Handler: ?Sized, Args: Clone> Clone for Command<Handler, Args> {
    #[inline]
    fn clone(&self) -> Self {
        Self {
            _handler: PhantomData,
            args: self.args.clone(),
        }
    }
}

impl<Handler: ?Sized, Args: Copy> Copy for Command<Handler, Args> {}

impl<Handler, Args> fmt::Debug for Command<Handler, Args>
where
    Handler: ?Sized,
    Args: fmt::Debug + ?Sized,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Command")
            .field("_handler", &self._handler)
            .field("args", &&self.args)
            .finish()
    }
}

impl<Handler, Args> cqrs::Command for Command<Handler, Args>
where
    Handler: ?Sized,
    Args: cqrs::Command + ?Sized,
{
    type Aggregate = <Args as cqrs::Command>::Aggregate;

    #[inline]
    fn aggregate_id(
        &self,
    ) -> Option<&<Self::Aggregate as cqrs::Aggregate>::Id> {
        self.args.aggregate_id()
    }
}

impl<Handler, Args> es::Command for Command<Handler, Args>
where
    Handler: ?Sized,
    Args: es::Command + ?Sized,
{
    #[inline]
    fn expected_version(&self) -> Option<es::aggregate::Version> {
        self.args.expected_version()
    }
}


impl<Args: cqrs::Command + ?Sized> cqrs::Command for With<Args> {
    type Aggregate = <Args as cqrs::Command>::Aggregate;

    #[inline]
    fn aggregate_id(
        &self,
    ) -> Option<&<Self::Aggregate as cqrs::Aggregate>::Id> {
        self.0.aggregate_id()
    }
}

impl<Args: es::Command> es::Command for With<Args> {
    #[inline]
    fn expected_version(&self) -> Option<es::aggregate::Version> {
        self.0.expected_version()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct WhenAbsent<Args: ?Sized>(pub Args);

impl<Args: ?Sized> WhenAbsent<With<Args>> {
    #[inline]
    #[must_use]
    pub fn args(&self) -> &Args {
        self.0.args()
    }
}

impl<Args> WhenAbsent<With<Args>> {
    #[inline]
    #[must_use]
    pub fn into_args(self) -> Args {
        self.0.into_args()
    }
}

impl<Args: cqrs::Command + ?Sized> cqrs::Command for WhenAbsent<Args> {
    type Aggregate = <Args as cqrs::Command>::Aggregate;

    #[inline]
    fn aggregate_id(
        &self,
    ) -> Option<&<Self::Aggregate as cqrs::Aggregate>::Id> {
        self.0.aggregate_id()
    }
}

impl<Args: es::Command> es::Command for WhenAbsent<Args> {
    #[inline]
    fn expected_version(&self) -> Option<es::aggregate::Version> {
        self.0.expected_version()
    }
}

#[derive(Clone, Copy, Debug)]
pub struct And<Args: ?Sized>(pub Args);

pub struct Extracted<Args: ?Sized>(PhantomData<JoinHandle<Box<Args>>>);

impl<Args: ?Sized> Clone for Extracted<Args> {
    #[inline]
    fn clone(&self) -> Self {
        *self
    }
}

impl<Args: ?Sized> Copy for Extracted<Args> {}

impl<Args: ?Sized> fmt::Debug for Extracted<Args> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("Extracted").field(&self.0).finish()
    }
}

*/
