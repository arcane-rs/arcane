//! Abstractions and tools for [Event Sourcing].
//!
//! [Event Sourcing]: https://martinfowler.com/eaaDev/EventSourcing.html

pub mod adapter;
pub mod event;

#[doc(inline)]
pub use self::adapter::Adapter;

#[doc(inline)]
pub use self::event::{
    Event, Initialized as EventInitialized, Name as EventName, Raw as RawEvent,
    Sourced as EventSourced, Sourcing as EventSourcing,
    Version as EventVersion, Versioned as VersionedEvent,
};
