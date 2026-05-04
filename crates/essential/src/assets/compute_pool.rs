use std::sync::OnceLock;

use crate::tasks::task_pool::TaskPool;

static COMPUTE_TASK_POOL: OnceLock<TaskPool> = OnceLock::new();

pub struct ComputeTaskPool;

impl ComputeTaskPool {
    pub fn get_or_init<F>(f: F) -> &'static TaskPool
    where
        F: FnOnce() -> TaskPool,
    {
        COMPUTE_TASK_POOL.get_or_init(f)
    }
}
