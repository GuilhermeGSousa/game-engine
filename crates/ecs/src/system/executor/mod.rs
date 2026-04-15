use petgraph::graph::NodeIndex;

use crate::{system::graph::SystemDependencyGraph, World};

pub(crate) mod multi_thread;
pub(crate) mod single_thread;

pub trait SystemExecutor: Send + Sync {
    fn run(
        &self,
        graph: &mut SystemDependencyGraph,
        sorted_systems: &[NodeIndex],
        world: &mut World,
    );
}
