//! Abstractions and tools for [Event Sourcing].
//!
//! [Event Sourcing]: https://martinfowler.com/eaaDev/EventSourcing.html

pub mod adapter;
pub mod event;

#[doc(inline)]
pub use self::adapter::{
    Adapter as EventAdapter, Transformer as EventTransformer,
};

#[doc(inline)]
pub use self::event::{
    Event, Initial as InitialEvent, Initialized as EventInitialized,
    Name as EventName, Sourced as EventSourced, Sourcing as EventSourcing,
    Version as EventVersion, Versioned as VersionedEvent,
};
