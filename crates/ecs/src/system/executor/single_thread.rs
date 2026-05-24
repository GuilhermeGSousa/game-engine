use crate::{
    system::{executor::SystemExecutor, schedule::CompiledScheduleData},
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

    fn run(&mut self, compiled_data: &mut CompiledScheduleData, world: &mut World) {
        for &idx in &compiled_data.sorted_systems {
            compiled_data.systems[idx].run_and_apply(world);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        resource::{Res, Resource},
        system::{config::IntoSystemConfig, executor::single_thread::SingleThreadedExecutor},
        Schedule,
    };
    use std::sync::{Arc, Mutex};

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

        print!("{schedule:?}");

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
