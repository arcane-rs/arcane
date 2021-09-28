use std::{array, iter};

use arcana::es::{
    adapter::{self, strategy, strategy::Splitter, WithStrategy},
    event::Initial,
};
use either::Either;
use futures::{future, stream, StreamExt as _};

use crate::event;

impl adapter::Returning for Adapter {
    type Error = serde_json::Error;
    type Transformed = event::Email;
}

#[derive(Clone, Copy, Debug)]
pub struct Adapter;

impl WithStrategy<event::email::Added> for Adapter {
    type Strategy = strategy::Initialized;
}

impl WithStrategy<event::email::v2::AddedAndConfirmed> for Adapter {
    type Strategy =
        strategy::Split<Either<event::email::Added, event::email::Confirmed>>;
}

impl WithStrategy<event::chat::public::Created> for Adapter {
    type Strategy = strategy::Skip;
}

impl WithStrategy<event::chat::private::Created> for Adapter {
    type Strategy = strategy::Skip;
}

impl WithStrategy<event::chat::v1::Created> for Adapter {
    type Strategy = strategy::Skip;
}

impl WithStrategy<event::message::Posted> for Adapter {
    type Strategy = strategy::Skip;
}

impl WithStrategy<event::email::Confirmed> for Adapter {
    type Strategy = strategy::Skip;
}

impl
    WithStrategy<
        event::Raw<event::email::v2::AddedAndConfirmed, serde_json::Value>,
    > for Adapter
{
    type Strategy = strategy::Custom;
}

impl
    Splitter<
        event::email::v2::AddedAndConfirmed,
        Either<event::email::Added, event::email::Confirmed>,
    > for Adapter
{
    type Iterator = SplitEmail;

    fn split(
        &self,
        event: event::email::v2::AddedAndConfirmed,
    ) -> Self::Iterator {
        use either::{Left, Right};

        #[allow(clippy::option_if_let_else)] // use of moved value
        if let Some(confirmed_by) = event.confirmed_by {
            Right(array::IntoIter::new([
                Left(event::email::Added { email: event.email }),
                Right(event::email::Confirmed { confirmed_by }),
            ]))
        } else {
            Left(array::IntoIter::new([Left(event::email::Added {
                email: event.email,
            })]))
        }
    }
}

type SplitEmail = Either<
    array::IntoIter<Either<event::email::Added, event::email::Confirmed>, 1>,
    array::IntoIter<Either<event::email::Added, event::email::Confirmed>, 2>,
>;

impl<Ctx>
    strategy::Customize<
        event::Raw<event::email::v2::AddedAndConfirmed, serde_json::Value>,
        Ctx,
    > for Adapter
{
    type Error = serde_json::Error;
    type Transformed = Either<event::email::Added, event::email::Confirmed>;
    type TransformedStream<'out> = CustomizedStream;

    fn transform<'me, 'ctx, 'out>(
        &'me self,
        event: event::Raw<
            event::email::v2::AddedAndConfirmed,
            serde_json::Value,
        >,
        _context: &'ctx Ctx,
    ) -> Self::TransformedStream<'out>
    where
        'me: 'out,
        'ctx: 'out,
    {
        match serde_json::from_value::<event::email::v2::AddedAndConfirmed>(
            event.data,
        ) {
            Ok(ev) => {
                let ok: fn(_) -> _ = Ok;
                stream::iter(Adapter.split(ev).map(ok)).left_stream()
            }
            Err(err) => stream::once(future::ready(Err(err))).right_stream(),
        }
    }
}

type CustomizedStream = future::Either<
    stream::Iter<
        iter::Map<
            SplitEmail,
            fn(
                Either<event::email::Added, event::email::Confirmed>,
            ) -> Result<
                Either<event::email::Added, event::email::Confirmed>,
                serde_json::Error,
            >,
        >,
    >,
    stream::Once<
        future::Ready<
            Result<
                Either<event::email::Added, event::email::Confirmed>,
                serde_json::Error,
            >,
        >,
    >,
>;

impl From<Either<event::email::Added, event::email::Confirmed>>
    for event::Email
{
    fn from(ev: Either<event::email::Added, event::email::Confirmed>) -> Self {
        match ev {
            Either::Left(ev) => Initial(ev).into(),
            Either::Right(ev) => ev.into(),
        }
    }
}
