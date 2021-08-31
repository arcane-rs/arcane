#![feature(generic_associated_types)]

use std::{any::Any, array, convert::Infallible};

use arcana::es::{
    adapter::transformer::{self, strategy},
    EventAdapter as _, EventTransformer,
};
use derive_more::From;
use either::Either;
use futures::{stream, TryStreamExt as _};

#[tokio::main]
async fn main() {
    let ctx = 1_usize; // Can be any type in this example.
    let events = stream::iter::<[InputEmailEvents; 5]>([
        EmailConfirmed {
            confirmed_by: "1".to_string(),
        }
        .into(),
        EmailAdded {
            email: "2".to_string(),
        }
        .into(),
        EmailAddedAndConfirmed {
            email: "3".to_string(),
            confirmed_by: Some("3".to_string()),
        }
        .into(),
        EmailAddedAndConfirmed {
            email: "4".to_string(),
            confirmed_by: None,
        }
        .into(),
        EmailConfirmationSent.into(),
    ]);

    let collect = Adapter
        .transform_all(events, &ctx)
        .try_collect::<Vec<_>>()
        .await
        .unwrap();

    println!("context: {}\nevents:{:#?}", ctx, collect);
}

// Events definitions

#[derive(Debug)]
struct EmailConfirmationSent;

#[derive(Debug)]
struct EmailAddedAndConfirmed {
    email: String,
    confirmed_by: Option<String>,
}

#[derive(Debug)]
struct EmailAdded {
    email: String,
}

#[derive(Debug)]
struct EmailConfirmed {
    confirmed_by: String,
}

#[derive(Debug, From, EventTransformer)]
#[event(
    transformer(
        adapter = Adapter,
        into = EmailAddedOrConfirmed,
        context = dyn Any,
        err = Infallible,
    )
)]
enum InputEmailEvents {
    ConfirmationSent(EmailConfirmationSent),
    AddedAndConfirmed(EmailAddedAndConfirmed),
    Added(EmailAdded),
    Confirmed(EmailConfirmed),
}

#[derive(Debug, From)]
enum EmailAddedOrConfirmed {
    Added(EmailAdded),
    Confirmed(EmailConfirmed),
}

// Adapter implementations

struct Adapter;

impl transformer::WithStrategy<EmailConfirmationSent> for Adapter {
    type Strategy = strategy::Skip;
}

impl transformer::WithStrategy<EmailAdded> for Adapter {
    type Strategy = strategy::AsIs;
}

impl transformer::WithStrategy<EmailConfirmed> for Adapter {
    // In this case can also be strategy::AsIs.
    type Strategy = strategy::Into<EmailAddedOrConfirmed>;
}

impl transformer::WithStrategy<EmailAddedAndConfirmed> for Adapter {
    type Strategy = strategy::Split<Either<EmailAdded, EmailConfirmed>>;
}

impl
    strategy::Splitter<
        EmailAddedAndConfirmed,
        Either<EmailAdded, EmailConfirmed>,
    > for Adapter
{
    type Iterator = SplitEmail;

    fn split(&self, event: EmailAddedAndConfirmed) -> Self::Iterator {
        use either::{Left, Right};

        #[allow(clippy::option_if_let_else)] // use of moved value
        if let Some(confirmed_by) = event.confirmed_by {
            Right(array::IntoIter::new([
                Left(EmailAdded { email: event.email }),
                Right(EmailConfirmed { confirmed_by }),
            ]))
        } else {
            Left(array::IntoIter::new([Left(EmailAdded {
                email: event.email,
            })]))
        }
    }
}

type SplitEmail = Either<
    array::IntoIter<Either<EmailAdded, EmailConfirmed>, 1>,
    array::IntoIter<Either<EmailAdded, EmailConfirmed>, 2>,
>;

impl From<Either<EmailAdded, EmailConfirmed>> for EmailAddedOrConfirmed {
    fn from(val: Either<EmailAdded, EmailConfirmed>) -> Self {
        match val {
            Either::Left(added) => EmailAddedOrConfirmed::Added(added),
            Either::Right(conf) => EmailAddedOrConfirmed::Confirmed(conf),
        }
    }
}
