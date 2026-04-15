use super::IntoSystem;
use crate::{
    system::{
        access::SystemAccess,
        batch::ScheduledBatch,
        graph::{SystemDependencyGraph, SystemNode},
        BoxedSystem,
    },
    world::World,
    Resource,
};
use derive_more::{Deref, From};
use petgraph::graph::NodeIndex;

#[derive(Clone, Copy, Eq, Hash, PartialEq, Deref, From)]
pub(crate) struct SystemNodeIndex(NodeIndex);

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
pub struct Schedule {
    system_ids: Vec<SystemNodeIndex>,
    graph: SystemDependencyGraph,
}

impl Schedule {
    /// Creates an empty schedule.
    pub fn new() -> Schedule {
        Self {
            system_ids: Vec::new(),
            graph: SystemDependencyGraph::new(),
        }
    }

    /// Appends a system to the end of the schedule.
    pub fn add_system<M>(&mut self, system: impl IntoSystem<M> + 'static) -> &mut Self {
        self.add_scheduled_system(system.into_system());
        self
    }

    /// Appends an already-boxed [`ScheduledSystem`] to the schedule (builder style).
    fn add_scheduled_system(&mut self, system: BoxedSystem) -> &mut Self {
        let added_system_access = system.access();
        let added_system_index = self.graph.add_node(SystemNode::new(system)).into();

        // TODO: System can also have user defined dependencies!

        for node_index in &self.system_ids {
            if let Some(other_system) = self.graph.node_weight(**node_index) {
                if !SystemAccess::are_disjoint(&added_system_access, &other_system.access()) {
                    self.graph.add_edge(**node_index, added_system_index, ());
                }
            }
        }

        self.system_ids.push(added_system_index.into());
        self
    }

    fn compile_batches(self) -> Vec<ScheduledBatch> {
        // Compute in-degree of each node
        // let mut in_degrees = vec![0usize; self.systems.len()];
        // for system_id in &self.system_ids {
        //     for system_edge in self.graph.edges(**system_id) {
        //         let system_edge_target = self.graph.node_weight(system_edge.target()).unwrap();
        //         in_degrees[*system_edge_target] += 1;
        //     }
        // }

        // let mut ready_queue = Vec::new();
        // for system_index in 0..self.system_ids.len() {
        //     if in_degrees[system_index] == 0 {
        //         ready_queue.push(system_index);
        //     }
        // }

        let mut processed_nodes = 0;
        let mut batches = Vec::new();
        // while processed_nodes < self.system_ids.len() {
        //     let candidates = ready_queue.drain(..).collect::<Vec<_>>();
        //     let mut batch = ScheduledBatch::default();

        //     // Put all the disjoint systems in the same batch. Those that aren't will go on the next one
        //     for candidate_index in candidates {
        //         let candidate_system_access = &self.cached_access[candidate_index];
        //         if batch.is_disjoint_from(candidate_system_access) {
        //             batch.push(candidate_index, candidate_system_access.clone());

        //             // Unlock successors of this node
        //             for edge in self.graph.edges(*self.system_ids[candidate_index]) {
        //                 let successor = self.graph.node_weight(edge.target()).unwrap();
        //                 in_degrees[*successor] -= 1;
        //                 // In Degree zero: this node is ready to be added to the next batch
        //                 if in_degrees[*successor] == 0 {
        //                     ready_queue.push(*successor);
        //                 }
        //             }
        //             processed_nodes += 1;
        //         } else {
        //             unreachable!("This should never happen wtf");
        //         }
        //     }
        //     batches.push(batch);
        // }
        batches
    }

    pub fn compile(self) -> CompiledSchedule {
        todo!()
    }
}

pub struct CompiledSchedule {}

// No constructor methods here! Get this by compiling a schedule
impl CompiledSchedule {
    // This will likely take an executor as well
    pub fn run(&self, world: &mut World) {}
}

/// Identifies which phase of the per-frame update loop a system belongs to.
///
/// Systems within the same group run in insertion order.  Groups themselves run
/// in this fixed order each frame (driven by [`App::update`](crate::App::update)):
///
/// 1. **Startup** — runs once when [`App::finish_plugin_build`](crate::App::finish_plugin_build) is called.
/// 2. **FixedUpdate** — runs zero or more times per frame to catch up with a fixed time step.
/// 3. **LateFixedUpdate** — runs once after each FixedUpdate pass.
/// 4. **Update** — the main per-frame update phase.
/// 5. **LateUpdate** — cleanup/reaction phase (e.g. event flushing, transform propagation).
/// 6. **Render** — submits draw calls to the GPU.
/// 7. **LateRender** — post-render work (e.g. UI overlay).
///
/// TODO: This will live here until we abstract update groups away from schedules
#[derive(Hash, PartialEq, Eq)]
pub enum UpdateGroup {
    /// One-shot startup systems, run before the first frame.
    Startup,
    /// Main per-frame update.
    Update,
    /// Fixed-timestep physics/logic update.
    FixedUpdate,
    /// Runs after `Update` (e.g. transform propagation, event flushing).
    LateUpdate,
    /// Runs after each `FixedUpdate` pass.
    LateFixedUpdate,
    /// GPU render submission.
    Render,
    /// Post-render overlay (e.g. UI).
    LateRender,
}

#[derive(Resource, Default)]
pub struct Schedules {
    startup_schedule: Schedule,
    update_schedule: Schedule,
    fixed_update_schedule: Schedule,
    late_update_schedule: Schedule,
    late_fixed_update_schedule: Schedule,
    render_schedule: Schedule,
    late_render_schedule: Schedule,
}

impl Schedules {
    /// Registers a system in the given [`UpdateGroup`].
    pub fn add_system<M>(
        &mut self,
        update_group: UpdateGroup,
        system: impl IntoSystem<M> + 'static,
    ) {
        match update_group {
            UpdateGroup::Startup => self.startup_schedule.add_system(system),
            UpdateGroup::Update => self.update_schedule.add_system(system),
            UpdateGroup::FixedUpdate => self.fixed_update_schedule.add_system(system),
            UpdateGroup::LateUpdate => self.late_update_schedule.add_system(system),
            UpdateGroup::LateFixedUpdate => self.late_fixed_update_schedule.add_system(system),
            UpdateGroup::Render => self.render_schedule.add_system(system),
            UpdateGroup::LateRender => self.late_render_schedule.add_system(system),
        };
    }

    pub fn compile(self) -> CompiledSchedules {
        CompiledSchedules {
            startup_schedule: self.startup_schedule.compile(),
            update_schedule: self.update_schedule.compile(),
            fixed_update_schedule: self.fixed_update_schedule.compile(),
            late_update_schedule: self.late_update_schedule.compile(),
            late_fixed_update_schedule: self.late_fixed_update_schedule.compile(),
            render_schedule: self.render_schedule.compile(),
            late_render_schedule: self.late_render_schedule.compile(),
        }
    }
}
#[derive(Resource)]
pub struct CompiledSchedules {
    startup_schedule: CompiledSchedule,
    update_schedule: CompiledSchedule,
    fixed_update_schedule: CompiledSchedule,
    late_update_schedule: CompiledSchedule,
    late_fixed_update_schedule: CompiledSchedule,
    render_schedule: CompiledSchedule,
    late_render_schedule: CompiledSchedule,
}

impl CompiledSchedules {
    pub fn startup(&self, world: &mut World) {
        self.startup_schedule.run(world);
    }

    pub fn update(&self, world: &mut World) {
        self.update_schedule.run(world);
        self.late_update_schedule.run(world);
    }

    pub fn fixed_update(&self, world: &mut World) {
        self.fixed_update_schedule.run(world);
        self.late_fixed_update_schedule.run(world);
    }

    pub fn render(&self, world: &mut World) {
        self.render_schedule.run(world);
        self.late_render_schedule.run(world);
    }
}

#[cfg(test)]
mod tests {
    use crate::{Component, Query};

    use super::*;

    #[test]
    fn schedule_new() {
        let schedule = Schedule::new();
        assert_eq!(schedule.graph.node_count(), 2);
        assert_eq!(schedule.system_ids.len(), 0);
    }

    #[test]
    fn add_system_builder_style() {
        let mut schedule = Schedule::new();
        schedule.add_system(|| {}).add_system(|| {});

        assert_eq!(schedule.graph.node_count(), 2);
        assert_eq!(schedule.system_ids.len(), 2);
    }

    #[test]
    fn system_dependency_graph_creation() {
        let mut schedule = Schedule::new();
        schedule.add_system(|| {});

        assert_eq!(schedule.graph.node_count(), 1);
    }

    #[test]
    fn multiple_systems_added() {
        let mut schedule = Schedule::new();
        schedule
            .add_system(|| {})
            .add_system(|| {})
            .add_system(|| {});

        assert_eq!(schedule.graph.node_count(), 3);
        assert_eq!(schedule.system_ids.len(), 3);
        assert_eq!(schedule.graph.node_count(), 3);
    }

    #[test]
    fn system_batching_empty() {
        let schedule = Schedule::new();

        let batches = schedule.compile_batches();
        assert!(batches.is_empty())
    }

    #[derive(Component)]
    struct ComponentA;

    #[derive(Component)]
    struct ComponentB;

    #[test]
    fn system_batching_two_disjoint() {
        let mut schedule = Schedule::new();
        schedule
            .add_system(|_foo: Query<&ComponentA>| {})
            .add_system(|_foo: Query<&ComponentB>| {});
        let batches = schedule.compile_batches();
        assert!(batches.len() == 1)
    }

    #[test]
    fn system_batching_two_not_disjoint() {
        let mut schedule = Schedule::new();
        schedule
            .add_system(|_foo: Query<(&ComponentA, &ComponentB)>| {})
            .add_system(|_foo: Query<&mut ComponentA>| {});
        let batches = schedule.compile_batches();
        assert!(batches.len() == 2)
    }
}
