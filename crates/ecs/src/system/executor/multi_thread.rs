use concurrent_queue::ConcurrentQueue;
use fixedbitset::FixedBitSet;
use tasks::{compute_pool::ComputeTaskPool, task_pool::TaskPool};

use crate::{
    system::{
        executor::SystemExecutor,
        schedule::{is_sync_point, CompiledScheduleData},
    },
    utilities::SyncUnsafeCell,
    System, World,
};

#[allow(dead_code)]
pub struct MultiThreadedExecutor {
    ready_systems: FixedBitSet,
    pending_systems: FixedBitSet,
    running_systems: FixedBitSet,
    unapplied_systems: FixedBitSet,
    dependency_count: Vec<usize>,
    dependants: Vec<Vec<usize>>,
}

impl SystemExecutor for MultiThreadedExecutor {
    fn init(compiled_data: &CompiledScheduleData) -> Self
    where
        Self: Sized,
    {
        let sys_count = compiled_data.systems.len();
        let pending_systems = FixedBitSet::with_capacity(sys_count);
        let ready_systems = FixedBitSet::with_capacity(sys_count);
        let running_systems = FixedBitSet::with_capacity(sys_count);
        let unapplied_systems = FixedBitSet::with_capacity(sys_count);

        Self {
            ready_systems,
            pending_systems,
            running_systems,
            unapplied_systems,
            dependency_count: compiled_data.dependency_count.clone(),
            dependants: compiled_data.dependants.clone(),
        }
    }

    fn run(&mut self, compiled_data: &mut CompiledScheduleData, world: &mut World) {
        let sys_count = compiled_data.systems.len();

        if sys_count == 0 {
            return;
        }

        let systems =
            SyncUnsafeCell::from_mut(compiled_data.systems.as_mut_slice()).as_slice_of_cells();
        let world_cell = world.as_unsafe_world_cell_mut();
        let queue = ConcurrentQueue::bounded(sys_count);

        self.pending_systems.set_range(.., true);
        self.ready_systems.clear();
        self.running_systems.clear();
        self.unapplied_systems.clear();
        self.dependency_count
            .clone_from(&compiled_data.dependency_count);

        ComputeTaskPool::get_or_init(TaskPool::new).scope(|scope| {
            while !self.pending_systems.is_clear() || !self.running_systems.is_clear() {
                self.update_ready_systems();

                for ready_system_idx in self.ready_systems.ones() {
                    self.running_systems.set(ready_system_idx, true);
                    let sys = unsafe { &mut *systems[ready_system_idx].get() };
                    let queue = &queue;
                    scope.spawn(async move {
                        unsafe {
                            sys.run_unsafe(world_cell);
                        }
                        queue
                            .push(ready_system_idx)
                            .expect("Error registering finished system");
                    });
                }
                self.ready_systems.clear();

                let mut apply_deferred = false;
                while let Ok(finished_system) = queue.pop() {
                    self.running_systems.set(finished_system, false);
                    self.unapplied_systems.set(finished_system, true);
                    let sys: &mut Box<dyn System + 'static> =
                        unsafe { &mut *systems[finished_system].get() };
                    if is_sync_point(sys) {
                        apply_deferred = true;
                    }

                    for &dependant in &self.dependants[finished_system] {
                        self.dependency_count[dependant] -= 1;
                    }
                }
                if apply_deferred {
                    for unapplied_system in self.unapplied_systems.ones() {
                        let sys = unsafe { &mut *systems[unapplied_system].get() };
                        sys.apply(world_cell.world_mut());
                    }
                    self.unapplied_systems.clear();
                }
            }
        });
    }
}

impl MultiThreadedExecutor {
    fn update_ready_systems(&mut self) {
        self.dependency_count
            .iter()
            .enumerate()
            .for_each(|(index, &count)| {
                if count == 0 && self.pending_systems[index] {
                    self.ready_systems.set(index, true);
                    self.pending_systems.set(index, false);
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        command::CommandQueue,
        component::Component,
        entity::Entity,
        query::Query,
        resource::ResMut,
        system::{executor::multi_thread::MultiThreadedExecutor, schedule::Schedule},
        Changed, Resource, World,
    };

    #[derive(Component)]
    struct TagComponent;

    #[derive(Resource)]
    struct Counter(usize);

    fn spawn_tag_entity(mut cmd: CommandQueue) {
        cmd.spawn(TagComponent);
    }

    fn check_changed_entities(
        query: Query<Entity, Changed<TagComponent>>,
        mut counter: ResMut<Counter>,
    ) {
        if query.iter().count() != 0 {
            counter.0 += 1;
        }
    }

    #[test]
    fn check_changed_after_spawned() {
        let mut world = World::new();
        world.insert_resource(Counter(0));

        let mut schedule = Schedule::new();
        schedule.add_system(spawn_tag_entity);
        schedule.add_system(check_changed_entities);

        println!("{:?}", schedule);

        schedule.compile::<MultiThreadedExecutor>().run(&mut world);

        assert_eq!(world.get_resource::<Counter>().unwrap().0, 1);
    }
}
