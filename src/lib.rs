#![doc = include_str!("../README.md")]
#![deny(
    nonstandard_style,
    rust_2018_idioms,
    rustdoc::broken_intra_doc_links,
    rustdoc::private_intra_doc_links,
    trivial_casts,
    trivial_numeric_casts
)]
#![forbid(non_ascii_idents, unsafe_code)]
#![warn(
    deprecated_in_future,
    missing_copy_implementations,
    missing_debug_implementations,
    missing_docs,
    unreachable_pub,
    unused_import_braces,
    unused_labels,
    unused_qualifications,
    unused_results
)]

#[doc(hidden)]
pub mod private;

#[doc(inline)]
pub use arcana_core::{
    Event, EventInitialized, EventName, EventSourced, EventVersion,
    InitialEvent, VersionedEvent,
};

/// Macro for deriving [`Event`](trait@Event) on enums. For structs consider
/// [`VersionedEvent`](macro@VersionedEvent).
///
/// This macro ensures that every combination of [`Event::name()`](trait@Event)
/// and [`Event::ver()`](trait@Event) are unique. The only limitation is that
/// every underlying [`Event`](trait@Event) or
/// [`VersionedEvent`](trait@VersionedEvent) impls should be generated with proc
/// macros.
///
/// # Attribute arguments
///
/// - `#[event(skip(check_unique_name_and_ver))]` — optional
///
///   Use this value on whole container or particular enum variant to skip check
///   for unique combination of [`Event::name()`](trait@Event) and
///   [`Event::ver()`](trait@Event).
///
/// # Examples
///
/// ```compile_fail
/// # use arcana::{Event, VersionedEvent};
/// #
/// #[derive(VersionedEvent)]
/// #[event(name = "chat", version = 1)]
/// struct ChatEvent;
///
/// #[derive(VersionedEvent)]
/// #[event(name = "file", version = 1)]
/// struct FileEvent;
///
/// #[derive(Event)]
/// enum AnyEvent {
///     Chat(ChatEvent),
///     File { event: FileEvent },
/// }
///
/// #[derive(Event)]
/// enum DuplicatedEvent {
///     Any(AnyEvent),
///     File(FileEvent),
/// }
/// ```
///
/// ```
/// # use arcana::{Event, VersionedEvent};
/// #
/// # #[derive(VersionedEvent)]
/// # #[event(name = "chat", version = 1)]
/// # struct ChatEvent;
/// #
/// # #[derive(VersionedEvent)]
/// # #[event(name = "file", version = 1)]
/// # struct FileEvent;
/// #
/// # #[derive(Event)]
/// # enum AnyEvent {
/// #     Chat(ChatEvent),
/// #     File { event: FileEvent },
/// # }
/// #
/// #[derive(Event)]
/// enum DuplicatedEvent {
///     Any(AnyEvent),
///     #[event(skip(check_unique_name_and_ver))]
///     File(FileEvent),
/// }
/// ```
#[cfg(feature = "derive")]
pub use arcana_codegen::Event;

/// Macro for deriving [`VersionedEvent`](trait@VersionedEvent) on structs. For
/// enums, consisting of different events consider [`Event`](macro@Event).
///
/// # Attribute arguments
///
/// - `#[event(name = "...")]` — required
///
///   Value used in [`VersionedEvent::name()`](trait@VersionedEvent) impl.
///
/// - `#[event(ver = NonZeroU16)]` — required
///
///   Value used in [`VersionedEvent::ver()`](trait@VersionedEvent) impl.
///
/// # Examples
///
/// ```
/// # use arcana::VersionedEvent;
/// #
/// #[derive(VersionedEvent)]
/// #[event(name = "event", version = 1)]
/// struct Event;
/// ```
#[cfg(feature = "derive")]
pub use arcana_codegen::VersionedEvent;
