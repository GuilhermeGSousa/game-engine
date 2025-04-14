use std::sync::OnceLock;

use crate::tasks::task_pool::TaskPool;

static LOAD_TASK_POOL: OnceLock<TaskPool> = OnceLock::new();

pub struct LoadTaskPool;

impl LoadTaskPool {
    pub fn get_or_init<F>(f: F) -> &'static TaskPool
    where
        F: FnOnce() -> TaskPool,
    {
        LOAD_TASK_POOL.get_or_init(f)
    }
}
