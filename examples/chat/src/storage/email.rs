use std::{array, convert::Infallible};

use arcana::es::{
    adapter::{
        self,
        transformer::{self, strategy},
    },
    event::Initial,
};
use either::Either;

use crate::event;

impl adapter::WithError for Adapter {
    type Error = Infallible;
    type Transformed = event::Email;
}

#[derive(Clone, Copy, Debug)]
pub struct Adapter;

impl<Ctx> transformer::WithStrategy<event::email::Added, Ctx> for Adapter {
    type Strategy = strategy::Initialized;
}

impl<Ctx> transformer::WithStrategy<event::email::v1::AddedAndConfirmed, Ctx>
    for Adapter
{
    type Strategy =
        strategy::Split<Either<event::email::Added, event::email::Confirmed>>;
}

impl<Ctx> transformer::WithStrategy<event::chat::public::Created, Ctx>
    for Adapter
{
    type Strategy = strategy::Skip;
}

impl<Ctx> transformer::WithStrategy<event::chat::private::Created, Ctx>
    for Adapter
{
    type Strategy = strategy::Skip;
}

impl<Ctx> transformer::WithStrategy<event::chat::v1::Created, Ctx> for Adapter {
    type Strategy = strategy::Skip;
}

impl<Ctx> transformer::WithStrategy<event::message::Posted, Ctx> for Adapter {
    type Strategy = strategy::Skip;
}

impl<Ctx> transformer::WithStrategy<event::email::Confirmed, Ctx> for Adapter {
    type Strategy = strategy::Skip;
}

impl
    strategy::Splitter<
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
