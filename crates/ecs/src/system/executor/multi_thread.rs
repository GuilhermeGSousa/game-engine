use crate::system::{executor::SystemExecutor, schedule::CompiledScheduleData};

pub struct MultiThreadedExecutor {}

impl SystemExecutor for MultiThreadedExecutor {
    fn init() -> Self
    where
        Self: Sized,
    {
        todo!()
    }

    fn run(
        &self,
        graph: &mut crate::system::graph::SystemDependencyGraph,
        compiled_data: &CompiledScheduleData,
        world: &mut crate::World,
    ) {
        todo!()
    }
}
