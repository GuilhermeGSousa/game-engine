use crate::{system::SystemFunction, world::World};

#[derive(Default)]
pub(crate) struct Scheduler {
    systems: Vec<Box<dyn Fn(&mut World)>>,
}

impl Scheduler {
    pub fn add_system<F, Args>(mut self, system: F) -> Self
    where
        F: SystemFunction<Args> + 'static,
    {
        self.systems.push(Box::new(move |world| system.run(world)));
        self
    }

    pub fn run(&self, world: &mut World) {
        for system in &self.systems {
            system(world);
        }
    }
}
