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
    type Data<'world, 'state> = EventWriter<'world, T>;

    fn init_state() -> Self::State {
        ()
    }

    unsafe fn get_data<'world, 'state>(
        _state: &'state mut Self::State,
        world: crate::world::UnsafeWorldCell<'world>,
    ) -> Self::Data<'world, 'state> {
        EventWriter::new(world)
    }

    fn fill_access(access: &mut crate::system::access::SystemAccess) {
        access.write_resource::<EventChannel<T>>();
    }
}
