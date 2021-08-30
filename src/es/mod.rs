//! Abstractions and tools for [Event Sourcing].
//!
//! [Event Sourcing]: https://martinfowler.com/eaaDev/EventSourcing.html

pub mod event;

#[doc(inline)]
pub use self::event::{
    Event, Initial as InitialEvent, Initialized as EventInitialized,
    Name as EventName, Sourced as EventSourced, Version as EventVersion,
    Versioned as VersionedEvent,
};
