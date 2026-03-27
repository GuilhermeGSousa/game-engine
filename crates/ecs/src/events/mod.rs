pub mod event_channel;
pub mod event_reader;
pub mod event_writer;

pub use ecs_macros::Event;

/// Marker trait for messages that can be sent between systems.
///
/// Derive with `#[derive(Event)]`.  Register a new event type with the app's
/// `register_event` method before using it.
///
/// Send events from systems with [`EventWriter`](event_writer::EventWriter) and
/// read them with [`EventReader`](event_reader::EventReader).
///
/// Events are buffered for one frame and flushed at the end of `LateUpdate`.
///
/// # Example
/// ```ignore
/// use ecs::events::Event;
///
/// #[derive(Event)]
/// struct PlayerDied { score: u32 }
/// ```
pub trait Event: Send + Sync {}
