use std::ops::{Deref, DerefMut};

use crate::world::UnsafeWorldCell;
use typle::typle;

pub unsafe trait SystemInput {
    type State: 'static;
    type Data<'world>;

    fn init_state() -> Self::State;

    unsafe fn get_data<'world>(world: UnsafeWorldCell<'world>) -> Self::Data<'world>;
}

#[allow(unused_variables)]
#[typle(Tuple for 0..=12)]
unsafe impl<T> SystemInput for T
where
    T: Tuple,
    T<_>: SystemInput + 'static,
{
    type State = typle_for!(i in .. => T<{i}>::State);
    type Data<'world> = typle_for!(i in .. => T<{i}>::Data<'world>);

    fn init_state() -> Self::State {
        typle_for!(i in .. => <T<{i}>>::init_state())
    }

    unsafe fn get_data<'world>(world: UnsafeWorldCell<'world>) -> Self::Data<'world> {
        typle_for!(i in .. => <T<{i}>>::get_data(world))
    }
}

pub type SystemInputData<'w, P> = <P as SystemInput>::Data<'w>;

pub struct StaticSystemInput<'w, P: SystemInput>(SystemInputData<'w, P>);

impl<'w, P: SystemInput> Deref for StaticSystemInput<'w, P> {
    type Target = SystemInputData<'w, P>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'w, P: SystemInput> DerefMut for StaticSystemInput<'w, P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'w, P: SystemInput> StaticSystemInput<'w, P> {
    pub fn into_inner(self) -> SystemInputData<'w, P> {
        self.0
    }
}

unsafe impl<'w, P: SystemInput + 'static> SystemInput for StaticSystemInput<'w, P> {
    type State = P::State;
    type Data<'world> = StaticSystemInput<'world, P>;

    fn init_state() -> Self::State {
        P::init_state()
    }

    unsafe fn get_data<'world>(world: UnsafeWorldCell<'world>) -> Self::Data<'world> {
        StaticSystemInput(unsafe { P::get_data(world) })
    }
}
