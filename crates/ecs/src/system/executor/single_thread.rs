use crate::{
    system::{executor::SystemExecutor, schedule::CompiledSchedule},
    World,
};

pub struct SingleThreadedExecutor {}

impl SystemExecutor for SingleThreadedExecutor {
    fn run(&self, schedule: &mut CompiledSchedule, world: &mut World) {}
}
