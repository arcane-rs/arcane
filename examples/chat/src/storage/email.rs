use std::{any::Any, array, convert::Infallible};

use arcana::es::{
    adapter::{
        transformer::{self, strategy},
        Transformer,
    },
    event::Initial,
};
use either::Either;

use crate::event;

#[derive(Debug, Transformer)]
#[event(
    transformer(
        from(
            (super::Event, number_of_events = 3),
            (super::MessageEvent, number_of_events = 1),
            (super::ChatEvent, number_of_events = 3),
            (super::EmailEvent, number_of_events = 3),
        ),
    into = event::Email,
    context = dyn Any,
    error = Infallible,
    ),
)]
pub struct Adapter;

impl transformer::WithStrategy<event::email::v1::AddedAndConfirmed>
    for Adapter
{
    type Strategy =
        strategy::Split<Either<event::email::Added, event::email::Confirmed>>;
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
