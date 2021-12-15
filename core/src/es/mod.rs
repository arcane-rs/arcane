//! Abstractions and tools for [Event Sourcing].
//!
//! [Event Sourcing]: https://martinfowler.com/eaaDev/EventSourcing.html

pub mod event;

#[doc(inline)]
pub use self::event::{
    adapter::Adapter as EventAdapter, Event, Initialized as EventInitialized,
    Name as EventName, Raw as RawEvent, Sourced as EventSourced,
    Sourcing as EventSourcing, Version as EventVersion,
    Versioned as VersionedEvent,
};
