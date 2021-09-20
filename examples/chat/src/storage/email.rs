use std::{array, convert::Infallible};

use arcana::es::{
    adapter::{
        self,
        transformer::strategy::{AsIs, Initialized, Skip, Split, Splitter},
        Transformer,
    },
    event::Initial,
};
use either::Either;

use crate::event;

impl<Ctx> adapter::WithError<Ctx> for Adapter {
    type Error = Infallible;
    type Transformed = event::Email;
}

#[derive(Debug, Transformer)]
#[transformer(
    Initialized => event::email::Added,
    AsIs => event::email::Confirmed,
    Skip => (
        event::chat::public::Created,
        event::chat::private::Created,
        event::chat::v1::Created,
        event::message::Posted,
    ),
    Split<Either<event::email::Added, event::email::Confirmed>> => (
        event::email::v1::AddedAndConfirmed,
    ),
)]
pub struct Adapter;

impl
    Splitter<
        event::email::v1::AddedAndConfirmed,
        Either<event::email::Added, event::email::Confirmed>,
    > for Adapter
{
    type Iterator = SplitEmail;

    fn split(
        &self,
        event: event::email::v1::AddedAndConfirmed,
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
