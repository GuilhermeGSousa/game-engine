use std::marker::PhantomData;

use crate::{system_input::SystemInput, world::UnsafeWorldCell};

use typle::typle;

pub type BoxedSystem = Box<dyn System>;

pub trait System {
    fn run<'world>(&mut self, world: UnsafeWorldCell<'world>);
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
    fn run<'world>(&mut self, world: UnsafeWorldCell<'world>) {
        self.system.run(world);
    }
}

pub struct FunctionSystem<F, Input> {
    pub func: F,
    _marker: PhantomData<Input>,
}

impl<F, Input> FunctionSystem<F, Input> {
    pub fn new(func: F) -> Self {
        Self {
            func,
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
    for<'a> F: FnMut(typle_args!(i in .. => T<{i}>))
        + FnMut(typle_args!(i in .. => T<{i}>::Data<'a>))
        + 'static,
{
    fn run<'world>(&mut self, world: UnsafeWorldCell<'world>) {
        (self.func)(typle_args!(i in .. => unsafe { <T<{i}>>::get_data(world) } ));
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
    for<'a> F: FnMut(typle_args!(i in .. => T<{i}>))
        + FnMut(typle_args!(i in .. => T<{i}>::Data<'a>))
        + 'static,
{
    fn into_system(self) -> ScheduledSystem {
        ScheduledSystem::new(FunctionSystem::new(self))
    }
}
