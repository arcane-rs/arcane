#![feature(generic_associated_types)]
#![feature(more_qualified_paths)]

use std::{any::Any, array, convert::Infallible};

use arcana::es::{
    adapter::{
        transformer::{self, strategy},
        Adapter, Transformer,
    },
    event, Event,
};
use derive_more::From;
use futures::{stream, TryStreamExt as _};

#[tokio::main]
async fn main() {
    let ctx = 1_usize; // Can be any type in this example.
    let events = stream::iter::<[InputEmailEvents; 5]>([
        InnerInputEvents::from(EmailConfirmed {
            confirmed_by: "1".to_string(),
        })
        .into(),
        InnerInputEvents::from(EmailAdded {
            email: "2".to_string(),
        })
        .into(),
        InnerInputEvents::from(EmailAddedAndConfirmed {
            email: "3".to_string(),
            confirmed_by: "3".to_string(),
        })
        .into(),
        SkippedEvent.into(),
        Custom.into(),
    ]);

    let collect = EmailAdapter
        .transform_all(events, &ctx)
        .try_collect::<Vec<_>>()
        .await
        .unwrap();

    println!("context: {}\nevents: {:?}", ctx, collect);
}

// Individual events

#[derive(Debug, event::Versioned)]
#[event(name = "custom", version = 1)]
struct Custom;

#[derive(Debug, event::Versioned)]
#[event(name = "skipped", version = 1)]
struct SkippedEvent;

#[derive(Debug, event::Versioned)]
#[event(name = "email.added_and_confirmed", version = 1)]
struct EmailAddedAndConfirmed {
    email: String,
    confirmed_by: String,
}

#[derive(Debug, event::Versioned)]
#[event(name = "email.added", version = 1)]
struct EmailAdded {
    email: String,
}

#[derive(Debug, event::Versioned)]
#[event(name = "email.confirmed", version = 1)]
struct EmailConfirmed {
    confirmed_by: String,
}

// Input events enum

#[derive(Debug, Event, From)]
enum InputEmailEvents {
    Custom(Custom),
    Skipped(SkippedEvent),
    Inner(InnerInputEvents),
}

#[derive(Debug, Event, From)]
enum InnerInputEvents {
    AddedAndConfirmed(EmailAddedAndConfirmed),
    Added(EmailAdded),
    Confirmed(EmailConfirmed),
}

// Output events enum

#[derive(Debug, Event, From)]
enum EmailAddedOrConfirmed {
    Added(EmailAdded),
    Confirmed(event::Initial<EmailConfirmed>),
}

// Adapter

#[derive(Transformer)]
#[event(
    transformer(
        from(InputEmailEvents, InnerInputEvents),
        into = EmailAddedOrConfirmed,
        ctx = dyn Any,
        err = Infallible,
        number_of_events = 3,
    ),
)]
struct EmailAdapter;

impl transformer::WithStrategy<EmailAdded> for EmailAdapter {
    type Strategy = strategy::Skip;
}

impl transformer::WithStrategy<EmailAddedAndConfirmed> for EmailAdapter {
    type Strategy = strategy::Split<EmailAddedOrConfirmed>;
}

impl strategy::Splitter<EmailAddedAndConfirmed, EmailAddedOrConfirmed>
    for EmailAdapter
{
    type Iterator = array::IntoIter<EmailAddedOrConfirmed, 2>;

    fn split(&self, ev: EmailAddedAndConfirmed) -> Self::Iterator {
        array::IntoIter::new([
            EmailAdded { email: ev.email }.into(),
            event::Initial(EmailConfirmed {
                confirmed_by: ev.confirmed_by,
            })
            .into(),
        ])
    }
}
