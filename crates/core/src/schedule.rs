use crate::{
    system::{IntoSystem, ScheduledSystem, System},
    world::World,
};

#[derive(Default)]
#[allow(unused)]
pub(crate) struct Scheduler {
    systems: Vec<ScheduledSystem>,
}

impl Scheduler {
    #[allow(unused)]
    pub fn add_system<M>(mut self, system: impl IntoSystem<M> + 'static) -> Self {
        self.systems.push(system.into_system());
        self
    }

    pub fn add_scheduled_system(mut self, system: ScheduledSystem) -> Self {
        self.systems.push(system);
        self
    }

    #[allow(unused)]
    pub fn run(&mut self, world: &mut World) {
        for system in &mut self.systems {
            system.run(world.as_unsafe_world_cell_ref());
        }
    }
}
