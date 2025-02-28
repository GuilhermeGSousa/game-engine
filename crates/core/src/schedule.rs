use crate::{system::SystemFunction, world::World};

#[derive(Default)]
#[allow(unused)]
pub(crate) struct Scheduler {
    systems: Vec<Box<dyn Fn(&mut World)>>,
}

impl Scheduler {
    #[allow(unused)]
    pub fn add_system<F, Args>(mut self, system: F) -> Self
    where
        F: SystemFunction<Args> + 'static,
    {
        self.systems.push(Box::new(move |world| system.run(world)));
        self
    }

    #[allow(unused)]
    pub fn run(&self, world: &mut World) {
        for system in &self.systems {
            system(world);
        }
    }
}
