//! Abstractions and tools for [Event Sourcing].
//!
//! [Event Sourcing]: https://martinfowler.com/eaaDev/EventSourcing.html

pub mod event;

#[doc(inline)]
pub use self::event::{
    Concrete as ConcreteEvent, Event, Initialized as EventInitialized,
    Meta as EventMeta, Name as EventName, Revisable as RevisableEvent, Revision as EventRevision,
    RevisionOf as EventRevisionOf, Sourced as EventSourced,
    Sourcing as EventSourcing, Static as StaticEvent, Version as EventVersion,
};
