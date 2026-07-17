use crate::task::Task;
use std::future::Future;

pub struct TaskPool;

impl TaskPool {
    pub fn new() -> Self {
        TaskPool
    }

    /// Name is ignored on wasm: there are no worker threads to label.
    pub fn with_name(_name: &str) -> Self {
        Self::new()
    }

    pub fn spawn<T>(&self, future: impl Future<Output = T> + 'static) -> Task<T>
    where
        T: Send + 'static,
    {
        Task::new(future)
    }
}
