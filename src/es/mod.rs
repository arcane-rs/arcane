//! Abstractions and tools for [Event Sourcing].
//!
//! [Event Sourcing]: https://martinfowler.com/eaaDev/EventSourcing.html

pub mod event;

#[doc(inline)]
pub use self::event::{
    Event, Initialized as EventInitialized, Name as EventName,
    Sourced as EventSourced, Sourcing as EventSourcing,
    Version as EventVersion, Versioned as VersionedEvent,
};
