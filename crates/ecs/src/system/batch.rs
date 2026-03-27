use crate::{system::ScheduledSystem, world::UnsafeWorldCell, System};

pub(crate) struct SystemBatch {
    systems: Vec<ScheduledSystem>,
}

impl SystemBatch {
    pub(crate) fn run<'world>(&mut self, world: UnsafeWorldCell<'world>) {
        self.systems.iter_mut().for_each(|system| system.run(world));
    }
}
