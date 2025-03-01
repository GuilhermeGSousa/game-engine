use std::marker::PhantomData;

use crate::{
    bundle::ComponentBundle, query::Query, system_input::SystemInput, world::UnsafeWorldCell,
};
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

pub trait IntoSystem {
    fn into_system(self) -> ScheduledSystem;
}

impl<T: System + 'static> IntoSystem for T {
    fn into_system(self) -> ScheduledSystem {
        ScheduledSystem::new(self)
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

impl<F> System for FunctionSystem<F, ()>
where
    F: Fn() + 'static,
{
    fn run<'world>(&mut self, _world: UnsafeWorldCell<'world>) {
        (self.func)()
    }
}

// impl<F, T0: ComponentBundle> System for FunctionSystem<F, Query<'_, T0>>
// where
//     for<'a> F: FnMut(Query<'a, T0>) + 'static,
// {
//     fn run<'world>(&mut self, world: UnsafeWorldCell<'world>) {
//         (self.func)(Query::new(world))
//     }
// }

impl<F, T0> System for FunctionSystem<F, T0>
where
    for<'a> F: FnMut(T0) + FnMut(T0::Data<'a>) + 'static,
    T0: SystemInput,
{
    fn run<'world>(&mut self, world: UnsafeWorldCell<'world>) {
        unsafe {
            (self.func)(T0::get_data(world));
        }
    }
}
