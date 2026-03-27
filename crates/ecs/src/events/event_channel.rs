use crate::{
    events::Event,
    resource::{ResMut, Resource},
};

/// Internal storage for a single event type.
///
/// Events are buffered here and flushed once per frame by [`update_event_channel`].
/// Prefer the higher-level [`EventWriter`](super::event_writer::EventWriter) and
/// [`EventReader`](super::event_reader::EventReader) in system code.
#[derive(Resource)]
pub struct EventChannel<T: Event + 'static> {
    pub(crate) event_queue: Vec<T>,
}

impl<T: Event + 'static> EventChannel<T> {
    /// Creates an empty channel.
    pub fn new() -> Self {
        EventChannel {
            event_queue: Vec::new(),
        }
    }

    /// Enqueues `event` to be read by [`EventReader`](super::event_reader::EventReader)s this frame.
    pub fn push_event(&mut self, event: T) {
        self.event_queue.push(event);
    }

    /// Clears all buffered events.
    pub fn flush_events(&mut self) {
        self.event_queue.clear();
    }
}

/// System that flushes all events in a channel at the end of `LateUpdate`.
///
/// Registered automatically by [`App::register_event`].
pub fn update_event_channel<T: Event + 'static>(mut channel: ResMut<EventChannel<T>>) {
    channel.flush_events();
}
