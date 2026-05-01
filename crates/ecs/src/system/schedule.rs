use crate::{
    system::{
        access::SystemAccess,
        config::{IntoSystemConfig, SystemConfig},
        executor::SystemExecutor,
        graph::{SystemDependencyGraph, SystemNode},
    },
    world::World,
    Resource,
};
use derive_more::{Deref, From};
use petgraph::{algo::toposort, graph::NodeIndex, Direction};

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

    /// Appends a system (or [`SystemConfig`]) to the schedule.
    ///
    /// Accepts bare system functions as well as configured systems built with
    /// [`.after()`](IntoSystemConfig::after) / [`.before()`](IntoSystemConfig::before):
    ///
    /// ```
    /// # use ecs::{Schedule, IntoSystemConfig};
    /// # fn a() {} fn b() {} fn c() {}
    /// let mut schedule = Schedule::new();
    /// schedule
    ///     .add_system(a)
    ///     .add_system(b.after(a))
    ///     .add_system(c.after(b).before(a));
    /// ```
    pub fn add_system<M>(&mut self, system: impl IntoSystemConfig<M> + 'static) -> &mut Self {
        self.add_config(system.into_config());
        self
    }

    /// Registers a [`SystemConfig`] into the graph, recursively registering owned dep
    /// systems first, then wiring explicit ordering edges.  Returns the [`NodeIndex`] of
    /// the newly registered system (used internally for edge wiring in recursive calls).
    fn add_config(&mut self, config: SystemConfig) -> NodeIndex {
        // 1. Register "after" deps first (they must run before this system).
        let after_indices: Vec<NodeIndex> = config
            .after
            .into_iter()
            .map(|dep| self.add_config(dep))
            .collect();

        // 2. Register "before" deps (they must run after this system).
        let before_indices: Vec<NodeIndex> = config
            .before
            .into_iter()
            .map(|dep| self.add_config(dep))
            .collect();

        // 3. Register this system.
        let access = config.system.access();
        let node_idx = self.graph.add_node(SystemNode::new(config.system));

        // 4. Implicit edges from access-pattern conflicts.
        for node_index in &self.system_ids {
            if let Some(other_system) = self.graph.node_weight(**node_index) {
                if !SystemAccess::are_disjoint(&access, other_system.access()) {
                    self.graph.add_edge(**node_index, node_idx, ());
                }
            }
        }

        // 5. Explicit ordering edges.
        for dep_idx in after_indices {
            self.graph.add_edge(dep_idx, node_idx, ());
        }
        for dep_idx in before_indices {
            self.graph.add_edge(node_idx, dep_idx, ());
        }

        self.system_ids.push(node_idx.into());
        node_idx
    }

    pub fn compile<T: SystemExecutor + 'static>(self) -> CompiledSchedule {
        let sorted_systems = toposort(&self.graph, None)
            .expect("Cycle detected in schedule — check your .after()/.before() constraints")
            .into_iter()
            .map(SystemNodeIndex::from)
            .collect::<Vec<_>>();

        let dependency_count: Vec<usize> = sorted_systems
            .iter()
            .map(|idx| {
                self.graph
                    .neighbors_directed(**idx, Direction::Incoming)
                    .count()
            })
            .collect();

        let dependants: Vec<Vec<SystemNodeIndex>> = sorted_systems
            .iter()
            .map(|idx| {
                self.graph
                    .neighbors_directed(**idx, Direction::Outgoing)
                    .map(SystemNodeIndex::from)
                    .collect()
            })
            .collect();

        let system_access: Vec<SystemAccess> = sorted_systems
            .iter()
            .map(|idx| self.graph.node_weight(**idx).unwrap().access().clone())
            .collect();

        let compiled_data = CompiledScheduleData {
            sorted_systems,
            dependency_count,
            dependants,
            system_access,
        };

        CompiledSchedule {
            executor: Box::new(T::init()),
            compiled_data,
            graph: self.graph,
        }
    }
}

pub struct CompiledSchedule {
    executor: Box<dyn SystemExecutor>,
    compiled_data: CompiledScheduleData,
    graph: SystemDependencyGraph,
}

// No constructor methods here! Get this by compiling a schedule
impl CompiledSchedule {
    pub fn run(&mut self, world: &mut World) {
        self.executor
            .run(&mut self.graph, &self.compiled_data, world);
    }
}

pub struct CompiledScheduleData {
    pub sorted_systems: Vec<SystemNodeIndex>,
    pub dependency_count: Vec<usize>,
    pub dependants: Vec<Vec<SystemNodeIndex>>,
    pub system_access: Vec<SystemAccess>,
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
        system: impl IntoSystemConfig<M> + 'static,
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

    pub fn compile<T: SystemExecutor + 'static>(self) -> CompiledSchedules {
        CompiledSchedules {
            startup_schedule: self.startup_schedule.compile::<T>(),
            update_schedule: self.update_schedule.compile::<T>(),
            fixed_update_schedule: self.fixed_update_schedule.compile::<T>(),
            late_update_schedule: self.late_update_schedule.compile::<T>(),
            late_fixed_update_schedule: self.late_fixed_update_schedule.compile::<T>(),
            render_schedule: self.render_schedule.compile::<T>(),
            late_render_schedule: self.late_render_schedule.compile::<T>(),
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
    pub fn startup(&mut self, world: &mut World) {
        self.startup_schedule.run(world);
    }

    pub fn update(&mut self, world: &mut World) {
        self.update_schedule.run(world);
        self.late_update_schedule.run(world);
    }

    pub fn fixed_update(&mut self, world: &mut World) {
        self.fixed_update_schedule.run(world);
        self.late_fixed_update_schedule.run(world);
    }

    pub fn render(&mut self, world: &mut World) {
        self.render_schedule.run(world);
        self.late_render_schedule.run(world);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        resource::Res,
        system::{config::IntoSystemConfig, executor::single_thread::SingleThreadedExecutor},
    };
    use petgraph::graph::NodeIndex;
    use std::sync::{Arc, Mutex};

    // ── Existing tests ────────────────────────────────────────────────────────

    #[test]
    fn schedule_new() {
        let schedule = Schedule::new();
        assert_eq!(schedule.graph.node_count(), 0);
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
    fn compile_and_run() {
        let mut schedule = Schedule::new();
        schedule
            .add_system(|| print!("First"))
            .add_system(|| print!("First"))
            .add_system(|| print!("First"));

        schedule
            .compile::<SingleThreadedExecutor>()
            .run(&mut World::new());
    }

    // ── Graph structure tests ─────────────────────────────────────────────────
    //
    // These tests use zero-parameter (data-disjoint) functions so that only
    // explicit ordering edges appear in the graph.

    #[test]
    fn after_registers_dep_and_main() {
        fn dep() {}
        fn main_sys() {}

        let mut schedule = Schedule::new();
        schedule.add_system(main_sys.after(dep));

        // Both systems must be present in the graph.
        assert_eq!(schedule.graph.node_count(), 2);
        assert_eq!(schedule.system_ids.len(), 2);
    }

    #[test]
    fn after_creates_dep_to_main_edge() {
        fn dep() {}
        fn main_sys() {}

        let mut schedule = Schedule::new();
        schedule.add_system(main_sys.after(dep));

        // dep is registered first → NodeIndex(0); main second → NodeIndex(1).
        // The explicit ordering edge must run dep before main: 0 → 1.
        assert_eq!(schedule.graph.edge_count(), 1);
        assert!(schedule
            .graph
            .contains_edge(NodeIndex::new(0), NodeIndex::new(1)));
    }

    #[test]
    fn before_registers_dep_and_main() {
        fn main_sys() {}
        fn dep() {}

        let mut schedule = Schedule::new();
        schedule.add_system(main_sys.before(dep));

        assert_eq!(schedule.graph.node_count(), 2);
        assert_eq!(schedule.system_ids.len(), 2);
    }

    #[test]
    fn before_creates_main_to_dep_edge() {
        fn main_sys() {}
        fn dep() {}

        let mut schedule = Schedule::new();
        schedule.add_system(main_sys.before(dep));

        // dep is registered first (as a "before" dep) → NodeIndex(0).
        // main is registered second → NodeIndex(1).
        // The explicit ordering edge must run main before dep: 1 → 0.
        assert_eq!(schedule.graph.edge_count(), 1);
        assert!(schedule
            .graph
            .contains_edge(NodeIndex::new(1), NodeIndex::new(0)));
    }

    #[test]
    fn after_before_chain_has_correct_edges() {
        fn sys_a() {}
        fn sys_b() {} // main: a → b → c
        fn sys_c() {}

        let mut schedule = Schedule::new();
        // Registration order in add_config:
        //   1. after  deps → sys_a : NodeIndex(0)
        //   2. before deps → sys_c : NodeIndex(1)
        //   3. main   sys_b        : NodeIndex(2)
        //   edges: 0→2 (a before b) and 2→1 (b before c)
        schedule.add_system(sys_b.after(sys_a).before(sys_c));

        assert_eq!(schedule.graph.node_count(), 3);
        assert_eq!(schedule.graph.edge_count(), 2);
        assert!(schedule
            .graph
            .contains_edge(NodeIndex::new(0), NodeIndex::new(2))); // a → b
        assert!(schedule
            .graph
            .contains_edge(NodeIndex::new(2), NodeIndex::new(1))); // b → c
    }

    #[test]
    fn nested_after_chain_has_correct_edges() {
        fn sys_a() {}
        fn sys_b() {}
        fn sys_c() {}

        let mut schedule = Schedule::new();
        // c.after(b.after(a)):  a → b → c
        // Registration order (depth-first): sys_a(0), sys_b(1), sys_c(2)
        // Edges: 0→1, 1→2
        schedule.add_system(sys_c.after(sys_b.after(sys_a)));

        assert_eq!(schedule.graph.node_count(), 3);
        assert_eq!(schedule.graph.edge_count(), 2);
        assert!(schedule
            .graph
            .contains_edge(NodeIndex::new(0), NodeIndex::new(1)));
        assert!(schedule
            .graph
            .contains_edge(NodeIndex::new(1), NodeIndex::new(2)));
    }

    // ── Runtime execution-order tests ─────────────────────────────────────────
    //
    // ExecLog is a read-only resource (Res<ExecLog>) that holds an Arc<Mutex<…>>
    // for interior mutability.  Since both systems take Res (not ResMut), they are
    // considered data-disjoint and produce no implicit ordering edge — so only the
    // explicit .after() / .before() constraints determine the run order.
    //
    // This also exercises the toposort bug-fix: .before() inserts the dep node
    // first in system_ids (insertion order = wrong), and only the toposort
    // produces the correct execution order.

    #[derive(Resource)]
    struct ExecLog(Arc<Mutex<Vec<u8>>>);

    impl Default for ExecLog {
        fn default() -> Self {
            Self(Arc::new(Mutex::new(Vec::new())))
        }
    }

    #[test]
    fn after_dep_runs_before_main_at_runtime() {
        fn push_1(log: Res<ExecLog>) {
            log.0.lock().unwrap().push(1);
        }
        fn push_2(log: Res<ExecLog>) {
            log.0.lock().unwrap().push(2);
        }

        let shared = Arc::new(Mutex::new(Vec::<u8>::new()));
        let mut world = World::new();
        world.insert_resource(ExecLog(Arc::clone(&shared)));

        let mut schedule = Schedule::new();
        // push_2 must run after push_1
        schedule.add_system(push_2.after(push_1));
        schedule.compile::<SingleThreadedExecutor>().run(&mut world);

        assert_eq!(*shared.lock().unwrap(), vec![1, 2]);
    }

    #[test]
    fn before_main_runs_before_dep_at_runtime() {
        fn push_1(log: Res<ExecLog>) {
            log.0.lock().unwrap().push(1);
        }
        fn push_2(log: Res<ExecLog>) {
            log.0.lock().unwrap().push(2);
        }

        let shared = Arc::new(Mutex::new(Vec::<u8>::new()));
        let mut world = World::new();
        world.insert_resource(ExecLog(Arc::clone(&shared)));

        let mut schedule = Schedule::new();
        // push_1 must run before push_2.
        // push_2 is registered first (as the "before" dep → NodeIndex 0),
        // then push_1 (main → NodeIndex 1).  Insertion order would run
        // push_2 first — only the toposort fix produces the correct [1, 2] result.
        schedule.add_system(push_1.before(push_2));
        schedule.compile::<SingleThreadedExecutor>().run(&mut world);

        assert_eq!(*shared.lock().unwrap(), vec![1, 2]);
    }

    #[test]
    fn nested_chain_runs_in_correct_order() {
        fn push_1(log: Res<ExecLog>) {
            log.0.lock().unwrap().push(1);
        }
        fn push_2(log: Res<ExecLog>) {
            log.0.lock().unwrap().push(2);
        }
        fn push_3(log: Res<ExecLog>) {
            log.0.lock().unwrap().push(3);
        }

        let shared = Arc::new(Mutex::new(Vec::<u8>::new()));
        let mut world = World::new();
        world.insert_resource(ExecLog(Arc::clone(&shared)));

        let mut schedule = Schedule::new();
        // push_3.after(push_2.after(push_1)):  1 → 2 → 3
        schedule.add_system(push_3.after(push_2.after(push_1)));
        schedule.compile::<SingleThreadedExecutor>().run(&mut world);

        assert_eq!(*shared.lock().unwrap(), vec![1, 2, 3]);
    }
}
