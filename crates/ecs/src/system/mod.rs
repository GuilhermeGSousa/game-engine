pub mod schedule;
pub mod system_input;

use std::marker::PhantomData;

use system_input::SystemInput;
use typle::typle;

use crate::world::{UnsafeWorldCell, World};

pub type BoxedSystem = Box<dyn System>;

pub trait System {
    fn run<'world>(&mut self, world: UnsafeWorldCell<'world>) {
        self.run_without_apply(world);
        self.apply(world.world_mut());
    }

    fn run_without_apply<'world>(&mut self, world: UnsafeWorldCell<'world>);

    fn apply(&mut self, world: &mut World);
}

pub struct ScheduledSystem {
    system: BoxedSystem,
}

impl ScheduledSystem {
    pub fn new(system: impl System + 'static) -> Self {
        Self {
            system: Box::new(system),
        }
    }
}

impl System for ScheduledSystem {
    fn apply(&mut self, world: &mut World) {
        self.system.apply(world);
    }

    fn run_without_apply<'world>(&mut self, world: UnsafeWorldCell<'world>) {
        self.system.run_without_apply(world);
    }
}

pub struct FunctionSystem<F, Input: SystemInput> {
    pub func: F,
    system_state: Input::State,
    _marker: PhantomData<Input>,
}

impl<F, Input> FunctionSystem<F, Input>
where
    Input: SystemInput + 'static,
{
    pub fn new(func: F) -> Self {
        Self {
            func,
            system_state: Input::init_state(),
            _marker: PhantomData,
        }
    }
}

#[allow(unused_variables)]
#[typle(Tuple for 0..=12)]
impl<F, T> System for FunctionSystem<F, T>
where
    T: Tuple,
    T<_>: SystemInput + 'static,
    for<'w, 's> F: FnMut(typle_args!(i in .. => T<{i}>))
        + FnMut(typle_args!(i in .. => T<{i}>::Data<'w, 's>))
        + 'static,
{
    fn run<'world>(&mut self, world: UnsafeWorldCell<'world>) {}

    fn apply(&mut self, world: &mut World) {
        for typle_index!(i) in 0..T::LEN {
            <T<{ i }>>::apply(&mut self.system_state[[i]], world);
        }
    }

    fn run_without_apply<'world>(&mut self, world: UnsafeWorldCell<'world>) {
        (self.func)(
            typle_args!(i in .. => unsafe { <T<{i}>>::get_data(&mut self.system_state[[i]], world) } ),
        );
    }
}

pub trait IntoSystem<Marker> {
    fn into_system(self) -> ScheduledSystem;
}

#[typle(Tuple for 0..=12)]
impl<F, T> IntoSystem<T> for F
where
    T: Tuple,
    T<_>: SystemInput + 'static,
    for<'w, 's> F: FnMut(typle_args!(i in .. => T<{i}>))
        + FnMut(typle_args!(i in .. => T<{i}>::Data<'w, 's>))
        + 'static,
{
    fn into_system(self) -> ScheduledSystem {
        ScheduledSystem::new(FunctionSystem::new(self))
    }
}
