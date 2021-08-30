#![feature(generic_associated_types)]

use std::{any::Any, array, convert::Infallible};

use arcana::es::{
    adapter::{
        transformer::{self, strategy},
        Transformer,
    },
    Adapter as _,
};
use derive_more::From;
use either::Either;
use futures::{future, stream, StreamExt as _, TryStreamExt as _};

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

#[derive(Debug, From)]
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

struct Adapter;

impl transformer::WithStrategy<EmailConfirmationSent> for Adapter {
    type Strategy = strategy::Skip;
}

impl transformer::WithStrategy<EmailAdded> for Adapter {
    type Strategy = strategy::AsIs;
}

impl transformer::WithStrategy<EmailConfirmed> for Adapter {
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

// TODO: generate

impl From<strategy::Unknown> for EmailAddedOrConfirmed {
    fn from(u: strategy::Unknown) -> Self {
        match u {}
    }
}

macro_rules! transformed_stream {
    (
        $me: lifetime,
        $ctx: lifetime,
        $adapter: ty,
        $from: ty,
        $event: ty
        $(,)?
    ) => {
        IntoTransformedStream<$me, $ctx, $adapter, $event, $from>
    };
    (
        $me: lifetime,
        $ctx: lifetime,
        $adapter: ty,
        $from: ty,
        $event: ty,
        $( $event_tail: ty ),+
        $(,)?
    ) => {
        future::Either<
            IntoTransformedStream<$me, $ctx, $adapter, $event, $from>,
            transformed_stream!($me, $ctx, $adapter, $from, $( $event_tail ),+)
        >
    };
}

type IntoTransformedStream<'me, 'ctx, Adapter, Event, From> = stream::Map<
    <Adapter as Transformer<Event>>::TransformedStream<'me, 'ctx>,
    fn(
        Result<
            <Adapter as Transformer<Event>>::Transformed,
            <Adapter as Transformer<Event>>::Error,
        >,
    ) -> Result<
        <Adapter as Transformer<From>>::Transformed,
        <Adapter as Transformer<From>>::Error,
    >,
>;

impl Transformer<InputEmailEvents> for Adapter {
    type Context = dyn Any;
    type Error = Infallible;
    type Transformed = EmailAddedOrConfirmed;
    type TransformedStream<'me, 'ctx> = transformed_stream!(
        'me,
        'ctx,
        Adapter,
        InputEmailEvents,
        EmailConfirmationSent,
        EmailAddedAndConfirmed,
        EmailAdded,
        EmailConfirmed,
    );

    fn transform<'me, 'ctx>(
        &'me self,
        event: InputEmailEvents,
        context: &'ctx Self::Context,
    ) -> Self::TransformedStream<'me, 'ctx> {
        fn transform_result<Ok, Err, IntoOk, IntoErr>(
            res: Result<Ok, Err>,
        ) -> Result<IntoOk, IntoErr>
        where
            IntoOk: From<Ok>,
            IntoErr: From<Err>,
        {
            res.map(Into::into).map_err(Into::into)
        }

        match event {
            InputEmailEvents::ConfirmationSent(event) => {
                <Adapter as Transformer<EmailConfirmationSent>>::transform(
                    self, event, context,
                )
                .map(transform_result as fn(_) -> _)
                .left_stream()
            }
            InputEmailEvents::AddedAndConfirmed(event) => {
                <Adapter as Transformer<EmailAddedAndConfirmed>>::transform(
                    self, event, context,
                )
                .map(transform_result as fn(_) -> _)
                .left_stream()
                .right_stream()
            }
            InputEmailEvents::Added(event) => {
                <Adapter as Transformer<EmailAdded>>::transform(
                    self, event, context,
                )
                .map(transform_result as fn(_) -> _)
                .left_stream()
                .right_stream()
                .right_stream()
            }
            InputEmailEvents::Confirmed(event) => {
                <Adapter as Transformer<EmailConfirmed>>::transform(
                    self, event, context,
                )
                .map(transform_result as fn(_) -> _)
                .right_stream()
                .right_stream()
                .right_stream()
            }
        }
    }
}
