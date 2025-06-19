use crate::system::system_input::SystemInput;

pub struct Commands;

unsafe impl SystemInput for Commands {
    type Data<'world> = ();

    unsafe fn get_data<'world>(world: crate::world::UnsafeWorldCell<'world>) -> Self::Data<'world> {
        ()
    }
}
