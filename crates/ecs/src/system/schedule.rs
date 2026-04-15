use std::{cmp::max, collections::BinaryHeap};

use super::{IntoSystem, ScheduledSystem, System};
use crate::{system::access::SystemAccess, world::World};
use derive_more::{Deref, From};
use petgraph::{
    graph::{DiGraph, NodeIndex},
    visit::{EdgeRef, Topo, Walker},
};

pub(crate) type SystemDependencyGraph = DiGraph<usize, ()>;

#[derive(Clone, Copy, Eq, Hash, PartialEq, Deref, From)]
pub(crate) struct ScheduledSystemIndex(NodeIndex);

/// An ordered list of systems that are executed sequentially each time [`run`](Schedule::run) is called.
///
/// Systems are added with [`add_system`](Schedule::add_system) (appended to the end) or
/// [`add_system_first`](Schedule::add_system_first) (prepended to the front).
///
/// # Example
/// ```
/// use ecs::{Schedule, World, Component, Query};
///
/// #[derive(Component)]
/// struct Velocity(f32);
///
/// fn apply_velocity(query: Query<&Velocity>) {
///     for v in query.iter() { /* ... */ }
/// }
///
/// let mut schedule = Schedule::new();
/// schedule.add_system(apply_velocity);
/// ```
#[derive(Default)]
#[allow(unused)]
pub struct Schedule {
    systems: Vec<ScheduledSystem>,
    system_ids: Vec<ScheduledSystemIndex>,
    graph: SystemDependencyGraph,
}

impl Schedule {
    /// Creates an empty schedule.
    pub fn new() -> Schedule {
        Self {
            systems: Vec::new(),
            system_ids: Vec::new(),
            graph: SystemDependencyGraph::new(),
        }
    }

    /// Appends a system to the end of the schedule.
    pub fn add_system<M>(&mut self, system: impl IntoSystem<M> + 'static) -> &mut Self {
        let added_system = system.into_system();
        let added_system_access = added_system.access();
        let added_system_index = self.graph.add_node(self.systems.len()).into();

        // TODO: System can also have user defined dependencies!

        for node_index in &self.system_ids {
            if let Some(other_system) = self.graph.node_weight(**node_index) {
                let system_access = self.systems[*other_system].access();
                if !SystemAccess::are_disjoint(&system_access, &added_system_access) {
                    self.graph.add_edge(**node_index, added_system_index, ());
                }
            }
        }
        self.system_ids.push(added_system_index.into());
        self
    }

    /// Prepends a system to the front of the schedule so it runs before all others.
    #[allow(unused)]
    pub fn add_system_first<M>(&mut self, system: impl IntoSystem<M> + 'static) -> &mut Self {
        self.systems.insert(0, system.into_system());
        self
    }

    /// Appends an already-boxed [`ScheduledSystem`] to the schedule (builder style).
    pub fn add_scheduled_system(mut self, system: ScheduledSystem) -> Self {
        self.systems.push(system);
        self
    }

    /// Runs all systems in order against `world`.
    pub fn run(&mut self, world: &mut World) {
        for system in &mut self.systems {
            system.run(world.as_unsafe_world_cell_mut());
        }
    }

    fn compile_batches(&mut self) {
        // Compute rank of each node
        let topo = Topo::new(&self.graph);
        let topo_order = topo.iter(&self.graph).collect::<Vec<_>>();

        let mut system_ranks = vec![0usize; self.systems.len()];
        for node_index in topo_order {
            let system_index = self.graph.node_weight(node_index).unwrap();
            for node_edge in self.graph.edges(node_index) {
                let node_edge_target = self.graph.node_weight(node_edge.target()).unwrap();

                system_ranks[*system_index] = max(
                    system_ranks[*system_index],
                    1 + system_ranks[*node_edge_target],
                );
            }
        }

        // Compute in-degree of each node
        let mut in_degrees = vec![0usize; self.systems.len()];
        for system_id in &self.system_ids {
            for system_edge in self.graph.edges(**system_id) {
                let system_edge_target = self.graph.node_weight(system_edge.target()).unwrap();
                in_degrees[*system_edge_target] += 1;
            }
        }

        let mut ready_queue = BinaryHeap::new();
    }
}
