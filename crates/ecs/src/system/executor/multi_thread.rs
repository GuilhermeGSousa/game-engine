use crate::{
    system::{executor::SystemExecutor, schedule::CompiledSchedule},
    World,
};

pub struct MultiThreadedExecutor {}

impl SystemExecutor for MultiThreadedExecutor {
    fn run(&self, schedule: &mut CompiledSchedule, world: &mut World) {
        unimplemented!()
    }
}
