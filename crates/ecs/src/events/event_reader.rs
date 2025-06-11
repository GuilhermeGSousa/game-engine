use std::{marker::PhantomData, slice::Iter};

use crate::{
    events::{event_channel::EventChannel, Event},
    resource::Res,
    system::system_input::SystemInput,
    world::UnsafeWorldCell,
};

pub struct EventReader<'world, T: Event + 'static> {
    channel: Res<'world, EventChannel<T>>,
}

impl<'w, T: Event> EventReader<'w, T> {
    pub fn new(world: UnsafeWorldCell<'w>) -> Self {
        Self {
            channel: Res::new(world),
        }
    }

    pub fn read(&self) -> EventIterator<T> {
        EventIterator::new(&self.channel)
    }
}

unsafe impl<'w, T> SystemInput for EventReader<'w, T>
where
    T: Event,
{
    type Data<'world> = EventReader<'world, T>;

    unsafe fn get_data<'world>(world: crate::world::UnsafeWorldCell<'world>) -> Self::Data<'world> {
        EventReader::new(world)
    }
}

pub struct EventIterator<'a, T: Event> {
    iter: Iter<'a, T>,
}

impl<'a, T: Event> EventIterator<'a, T> {
    pub fn new(channel: &'a EventChannel<T>) -> Self {
        Self {
            iter: channel.event_queue.iter(),
        }
    }
}

impl<'a, T: Event> Iterator for EventIterator<'a, T> {
    type Item = &'a T;

    fn next(&mut self) -> Option<Self::Item> {
        self.iter.next()
    }
}
