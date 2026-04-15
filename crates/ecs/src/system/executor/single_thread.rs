use petgraph::graph::NodeIndex;

use crate::{
    system::{executor::SystemExecutor, graph::SystemDependencyGraph},
    World,
};

pub struct SingleThreadedExecutor {}

impl SystemExecutor for SingleThreadedExecutor {
    fn run(
        &self,
        graph: &mut SystemDependencyGraph,
        sorted_systems: &[NodeIndex],
        world: &mut World,
    ) {
        for node_index in sorted_systems {
            graph
                .node_weight_mut(*node_index)
                .unwrap()
                .system
                .run(world.as_unsafe_world_cell_mut());
        }
    }
}
