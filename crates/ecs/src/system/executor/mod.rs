use crate::{system::schedule::CompiledSchedule, World};

pub(crate) mod multi_thread;
pub(crate) mod single_thread;

pub trait SystemExecutor: Send + Sync {
    fn run(&self, schedule: &mut CompiledSchedule, world: &mut World);
}
