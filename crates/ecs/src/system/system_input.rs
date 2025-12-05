use std::ops::{Deref, DerefMut};

use crate::{
    system::access::SystemAccess,
    world::{UnsafeWorldCell, World},
};
use typle::typle;

pub unsafe trait SystemInput {
    type State: 'static;
    type Data<'world, 'state>;

    fn init_state() -> Self::State;

    unsafe fn get_data<'world, 'state>(
        state: &'state mut Self::State,
        world: UnsafeWorldCell<'world>,
    ) -> Self::Data<'world, 'state>;

    fn apply(_state: &mut Self::State, _world: &mut World) {}

    fn fill_access(access: &mut SystemAccess);
}

#[allow(unused_variables, unused_mut)]
#[typle(Tuple for 0..=12)]
unsafe impl<T> SystemInput for T
where
    T: Tuple,
    T<_>: SystemInput + 'static,
{
    type State = typle_for!(i in .. => T<{i}>::State);
    type Data<'world, 'state> = typle_for!(i in .. => T<{i}>::Data<'world, 'state>);

    fn init_state() -> Self::State {
        typle_for!(i in .. => <T<{i}>>::init_state())
    }

    unsafe fn get_data<'world, 'state>(
        state: &'state mut Self::State,
        world: UnsafeWorldCell<'world>,
    ) -> Self::Data<'world, 'state> {
        typle_for!(i in .. => <T<{i}>>::get_data(&mut state[[i]], world))
    }

    fn fill_access(access: &mut SystemAccess) {
        typle_for!(i in .. => <T<{i}>>::fill_access(access));
    }
}

pub type SystemInputData<'w, 's, P> = <P as SystemInput>::Data<'w, 's>;

pub struct StaticSystemInput<'w, 's, P: SystemInput>(SystemInputData<'w, 's, P>);

impl<'w, 's, P: SystemInput> Deref for StaticSystemInput<'w, 's, P> {
    type Target = SystemInputData<'w, 's, P>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<'w, 's, P: SystemInput> DerefMut for StaticSystemInput<'w, 's, P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'w, 's, P: SystemInput> StaticSystemInput<'w, 's, P> {
    pub fn into_inner(self) -> SystemInputData<'w, 's, P> {
        self.0
    }
}

unsafe impl<'w, 's, P: SystemInput + 'static> SystemInput for StaticSystemInput<'w, 's, P> {
    type State = P::State;
    type Data<'world, 'state> = StaticSystemInput<'world, 'state, P>;

    fn init_state() -> Self::State {
        P::init_state()
    }

    unsafe fn get_data<'world, 'state>(
        state: &'state mut Self::State,
        world: UnsafeWorldCell<'world>,
    ) -> Self::Data<'world, 'state> {
        StaticSystemInput(unsafe { P::get_data(state, world) })
    }

    fn fill_access(access: &mut SystemAccess) {
        P::fill_access(access);
    }
}
