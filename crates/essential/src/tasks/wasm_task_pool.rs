use super::task::Task;
use std::future::Future;

pub struct TaskPool;

impl TaskPool {
    pub fn new() -> Self {
        TaskPool
    }

    pub fn spawn<T>(&self, future: impl Future<Output = T> + Send + 'static) -> Task<T>
    where
        T: Send + 'static,
    {
        wasm_bindgen_futures::spawn_local(async move {
            future.await;
        });

        Task::new()
    }
}
