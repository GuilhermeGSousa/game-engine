pub mod access;
pub mod schedule;
pub mod system_input;

use system_input::SystemInput;
use typle::typle;

use crate::{
    system::access::SystemAccess,
    world::{UnsafeWorldCell, World},
};

/// Type alias for a boxed, type-erased system.
pub type BoxedSystem = Box<dyn System>;

/// Core trait implemented by all executable systems.
///
/// In normal use you don't implement this directly — plain Rust functions whose
/// parameters implement [`SystemInput`] automatically implement [`IntoSystem`], which
/// wraps them in a [`FunctionSystem`] that implements `System`.
pub trait System: Send + Sync {
    /// Describes which components and resources this system reads or writes.
    fn access(&self) -> SystemAccess;

    /// Executes the system against the world, then applies any deferred commands.
    fn run<'world>(&mut self, world: UnsafeWorldCell<'world>) {
        self.run_without_apply(world);
        self.apply(world.world_mut());
    }

    /// Executes the system without applying deferred commands.
    fn run_without_apply<'world>(&mut self, world: UnsafeWorldCell<'world>);

    /// Applies any deferred mutations (e.g. spawned entities from [`CommandQueue`](crate::command::CommandQueue)).
    fn apply(&mut self, world: &mut World);
}

/// A type-erased, heap-allocated system ready to be stored in a [`Schedule`](schedule::Schedule).
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

    fn access(&self) -> SystemAccess {
        self.system.access()
    }
}

/// Wraps a plain function (or closure) and its cached input state into a [`System`].
pub struct FunctionSystem<F, Input: SystemInput> {
    pub func: F,
    system_state: Input::State,
}

impl<F, Input> FunctionSystem<F, Input>
where
    Input: SystemInput + 'static,
{
    pub fn new(func: F) -> Self {
        Self {
            func,
            system_state: Input::init_state(),
        }
    }
}

#[allow(unused_variables, unused_mut)]
#[typle(Tuple for 0..=12)]
impl<F, T> System for FunctionSystem<F, T>
where
    F: Send + Sync + 'static,
    T: Tuple,
    T<_>: SystemInput + 'static,
    for<'w, 's> F:
        FnMut(typle_args!(i in .. => T<{i}>)) + FnMut(typle_args!(i in .. => T<{i}>::Data<'w, 's>)),
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

    fn access(&self) -> SystemAccess {
        let mut access: SystemAccess = SystemAccess::default();
        for typle_index!(i) in 0..T::LEN {
            <T<{ i }>>::fill_access(&mut access);
        }
        access
    }
}

/// Conversion trait that turns a compatible function or closure into a [`ScheduledSystem`].
///
/// Implemented automatically for functions whose parameters implement [`SystemInput`].
pub trait IntoSystem<Marker> {
    /// Wraps `self` in a [`ScheduledSystem`] ready to be added to a [`Schedule`](schedule::Schedule).
    fn into_system(self) -> ScheduledSystem;
}

#[typle(Tuple for 0..=12)]
impl<F, T> IntoSystem<T> for F
where
    F: Send + Sync + 'static,
    T: Tuple,
    T<_>: SystemInput + 'static,
    for<'w, 's> F:
        FnMut(typle_args!(i in .. => T<{i}>)) + FnMut(typle_args!(i in .. => T<{i}>::Data<'w, 's>)),
{
    fn into_system(self) -> ScheduledSystem {
        ScheduledSystem::new(FunctionSystem::new(self))
    }
}
