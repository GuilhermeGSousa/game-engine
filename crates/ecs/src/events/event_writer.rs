use crate::{
    events::{event_channel::EventChannel, Event},
    resource::ResMut,
    system::system_input::SystemInput,
    world::UnsafeWorldCell,
};

pub struct EventWriter<'world, T: Event + 'static> {
    channel: ResMut<'world, EventChannel<T>>,
}

impl<'w, T: Event> EventWriter<'w, T> {
    pub fn new(world: UnsafeWorldCell<'w>) -> Self {
        Self {
            channel: ResMut::new(world),
        }
    }
    pub fn write(&mut self, event: T) {
        self.channel.push_event(event);
    }
}

unsafe impl<'w, T> SystemInput for EventWriter<'w, T>
where
    T: Event,
{
    type State = ();
    type Data<'world> = EventWriter<'world, T>;

    fn init_state() -> Self::State {
        ()
    }

    unsafe fn get_data<'world>(world: crate::world::UnsafeWorldCell<'world>) -> Self::Data<'world> {
        EventWriter::new(world)
    }
}
