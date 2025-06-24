use crate::{system::system_input::SystemInput, world::World};

pub struct CommandQueue;

unsafe impl SystemInput for CommandQueue {
    type Data<'world> = ();

    unsafe fn get_data<'world>(world: crate::world::UnsafeWorldCell<'world>) -> Self::Data<'world> {
        ()
    }
}

pub trait Command {
    fn execute(&self, world: &mut World);
}
