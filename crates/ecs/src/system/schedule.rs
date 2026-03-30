use crate::world::World;

use super::{IntoSystem, ScheduledSystem, System};

/// An ordered list of systems that are executed sequentially each time [`run`](Schedule::run) is called.
///
/// Systems are added with [`add_system`](Schedule::add_system) (appended to the end) or
/// [`add_system_first`](Schedule::add_system_first) (prepended to the front).
///
/// # Example
/// ```
/// use ecs::{Schedule, World, Component, Query};
///
/// #[derive(Component)]
/// struct Velocity(f32);
///
/// fn apply_velocity(query: Query<&Velocity>) {
///     for v in query.iter() { /* ... */ }
/// }
///
/// let mut schedule = Schedule::new();
/// schedule.add_system(apply_velocity);
/// ```
#[derive(Default)]
#[allow(unused)]
pub struct Schedule {
    systems: Vec<ScheduledSystem>,
}

impl Schedule {
    // TODO: Implement an actual scheduler with stages and conditions
    /// Creates an empty schedule.
    pub fn new() -> Schedule {
        Self {
            systems: Vec::new(),
        }
    }

    /// Appends a system to the end of the schedule.
    #[allow(unused)]
    pub fn add_system<M>(&mut self, system: impl IntoSystem<M> + 'static) -> &mut Self {
        let scheduled_system = system.into_system();
        let system_access = scheduled_system.access();
        self.systems.push(scheduled_system);
        self
    }

    /// Prepends a system to the front of the schedule so it runs before all others.
    #[allow(unused)]
    pub fn add_system_first<M>(&mut self, system: impl IntoSystem<M> + 'static) -> &mut Self {
        self.systems.insert(0, system.into_system());
        self
    }

    /// Appends an already-boxed [`ScheduledSystem`] to the schedule (builder style).
    pub fn add_scheduled_system(mut self, system: ScheduledSystem) -> Self {
        self.systems.push(system);
        self
    }

    /// Runs all systems in order against `world`.
    #[allow(unused)]
    pub fn run(&mut self, world: &mut World) {
        for system in &mut self.systems {
            system.run(world.as_unsafe_world_cell_mut());
        }
    }
}
