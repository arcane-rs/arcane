//! Abstractions and tools for [Event Sourcing].
//!
//! [Event Sourcing]: https://martinfowler.com/eaaDev/EventSourcing.html

pub mod event;

#[doc(inline)]
pub use self::event::{
    Event, Initialized as EventInitialized, Name as EventName,
    Revised as RevisedEvent, Revision as EventRevision,
    Sourced as EventSourced, Sourcing as EventSourcing, Meta as EventMeta,
};
