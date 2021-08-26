//! [`Event`] machinery.

#[doc(inline)]
pub use arcana_core::es::event::{
    Event, Initial, Initialized, Name, Sourced, Version, Versioned,
};

/// Macro for deriving [`Event`] on enums. For structs consider
/// [`Versioned`](macro@Versioned).
///
/// This macro ensures that every combination of [`Event::name()`][0] and
/// [`Event::version()`][1] are unique. The only limitation is that every
/// underlying [`Event`] or [`Versioned`](trait@Versioned) impls should be
/// generated with proc macros.
///
/// __Note__ may not work with complex generics with where clause because of
/// internal code generation. Should be resolved with [#57775][issue].
///
/// # Attribute arguments
///
/// - `#[event(skip)]` — optional
///
///   Use this value on particular enum variant to skip [`Event`]  impl for it
///   and check for unique combination of [`Event::name()`][0] and
///   [`Event::version()`][1].
///
///   __Note__: calling [`Event::name()`][0] or [`Event::version()`][1] on those
///   variants will result in [`unreachable!()`] panic.
///
/// # Examples
///
/// ```compile_fail,E0080
/// # use arcana::es::{Event, event};
/// #
/// #[derive(event::Versioned)]
/// #[event(name = "chat", version = 1)]
/// struct ChatEvent;
///
/// #[derive(event::Versioned)]
/// #[event(name = "file", version = 1)]
/// struct FileEvent;
///
/// #[derive(Event)]
/// enum AnyEvent {
///     Chat(ChatEvent),
///     File(FileEvent),
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
/// # use arcana::es::{Event, event};
/// #
/// # #[derive(event::Versioned)]
/// # #[event(name = "chat", version = 1)]
/// # struct ChatEvent;
/// #
/// # #[derive(event::Versioned)]
/// # #[event(name = "file", version = 1)]
/// # struct FileEvent;
/// #
/// # #[derive(Event)]
/// # enum AnyEvent {
/// #     Chat(ChatEvent),
/// #     File(FileEvent),
/// # }
/// #
/// #[derive(Event)]
/// enum DuplicatedEvent {
///     Any(AnyEvent),
///     #[event(skip)]
///     File(FileEvent),
/// }
/// ```
///
/// [0]: trait@Event::name()
/// [1]: trait@Event::version()
/// [issue]: https://github.com/rust-lang/rust/issues/57775
/// [`Event`]: trait@Event
#[cfg(feature = "derive")]
pub use arcana_codegen::es::event::Event;

/// Macro for deriving [`Versioned`](trait@Versioned) on structs. For
/// enums, consisting of different events consider [`Event`](macro@Event).
///
/// # Attribute arguments
///
/// - `#[event(name = "...")]` — required
///
///   Value used in [`Versioned::name()`][0] impl.
///
/// - `#[event(ver = NonZeroU16)]` — required
///
///   Value used in [`Versioned::version()`][1] impl.
///
/// # Examples
///
/// ```
/// # use arcana::es::event;
/// #
/// #[derive(event::Versioned)]
/// #[event(name = "event", version = 1)]
/// struct Event;
/// ```
///
/// [0]: trait@Versioned::name()
/// [1]: trait@Versioned::version()
#[cfg(feature = "derive")]
pub use arcana_codegen::es::event::Versioned;
