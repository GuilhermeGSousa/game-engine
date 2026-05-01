use crate::{
    system::{
        executor::SystemExecutor, graph::SystemDependencyGraph, schedule::CompiledScheduleData,
    },
    World,
};

pub struct SingleThreadedExecutor {}

impl SystemExecutor for SingleThreadedExecutor {
    fn init(_compiled_data: &CompiledScheduleData) -> Self
    where
        Self: Sized,
    {
        Self {}
    }

    fn run(
        &self,
        graph: &mut SystemDependencyGraph,
        compiled_data: &CompiledScheduleData,
        world: &mut World,
    ) {
        for node_index in &compiled_data.sorted_systems {
            graph
                .node_weight_mut(**node_index)
                .unwrap()
                .system
                .run_and_apply(world.as_unsafe_world_cell_mut());
        }
    }
}
