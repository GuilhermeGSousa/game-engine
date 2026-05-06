use tasks::{compute_pool::ComputeTaskPool, task_pool::TaskPool};

use crate::system::{
    executor::SystemExecutor,
    schedule::{CompiledScheduleData, SystemNodeIndex},
};

#[allow(dead_code)]
pub struct MultiThreadedExecutor {
    starting_systems: Vec<SystemNodeIndex>,
    dependency_count: Vec<usize>,
}

impl SystemExecutor for MultiThreadedExecutor {
    fn init(compiled_data: &CompiledScheduleData) -> Self
    where
        Self: Sized,
    {
        for system in &compiled_data.sorted_systems {
            if compiled_data.dependency_count[system.index()] == 0 {}
        }

        let starting_systems = compiled_data
            .sorted_systems
            .iter()
            .filter_map(|index| {
                if compiled_data.dependency_count[index.index()] == 0 {
                    Some(*index)
                } else {
                    None
                }
            })
            .collect();

        Self {
            starting_systems,
            dependency_count: compiled_data.dependency_count.clone(),
        }
    }

    fn run(
        &self,
        _graph: &mut crate::system::graph::SystemDependencyGraph,
        _compiled_data: &CompiledScheduleData,
        _world: &mut crate::World,
    ) {
        ComputeTaskPool::get_or_init(TaskPool::new).scope(|_scope: &tasks::task_pool::ScopedTaskPool<'_, ()>|
        {

        });
    }
}
