use crate::{system::system_input::SystemInput, world::World};

pub struct CommandQueue;

unsafe impl SystemInput for CommandQueue {
    type State = ();
    type Data<'world> = ();

    fn init_state() -> Self::State {
        ()
    }
    
    unsafe fn get_data<'world>(world: crate::world::UnsafeWorldCell<'world>) -> Self::Data<'world> {
        ()
    }
}

pub trait Command {
    fn execute(&self, world: &mut World);
}
