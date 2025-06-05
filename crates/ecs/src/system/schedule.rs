use crate::world::World;

use super::{IntoSystem, ScheduledSystem, System};

#[derive(Default)]
#[allow(unused)]
pub struct Schedule {
    systems: Vec<ScheduledSystem>,
}

impl Schedule {
    // TODO: Implement an actual scheduler with stages and conditions
    pub fn new() -> Schedule {
        Self {
            systems: Vec::new(),
        }
    }

    #[allow(unused)]
    pub fn add_system<M>(&mut self, system: impl IntoSystem<M> + 'static) -> &mut Self {
        self.systems.push(system.into_system());
        self
    }

    #[allow(unused)]
    pub fn add_system_first<M>(&mut self, system: impl IntoSystem<M> + 'static) -> &mut Self {
        self.systems.insert(0, system.into_system());
        self
    }

    pub fn add_scheduled_system(mut self, system: ScheduledSystem) -> Self {
        self.systems.push(system);
        self
    }

    #[allow(unused)]
    pub fn run(&mut self, world: &mut World) {
        for system in &mut self.systems {
            system.run(world.as_unsafe_world_cell_mut());
        }
    }
}
