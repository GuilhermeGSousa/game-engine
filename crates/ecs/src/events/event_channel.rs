use crate::{
    events::Event,
    resource::{ResMut, Resource},
};

#[derive(Resource)]
pub struct EventChannel<T: Event + 'static> {
    pub(crate) event_queue: Vec<T>,
}

impl<T: Event + 'static> EventChannel<T> {
    pub fn new() -> Self {
        EventChannel {
            event_queue: Vec::new(),
        }
    }

    pub fn push_event(&mut self, event: T) {
        self.event_queue.push(event);
    }

    pub fn flush_events(&mut self) {
        self.event_queue.clear();
    }
}

pub fn update_event_channel<T: Event + 'static>(mut channel: ResMut<EventChannel<T>>) {
    channel.flush_events();
}
