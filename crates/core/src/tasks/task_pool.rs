use std::future::Future;

use super::Task;

pub struct TaskPool;

impl TaskPool {
    pub fn spawn<T>(_task: impl Future<Output = T> + Send + 'static) -> Task<T>
    where
        T: 'static,
    {
        todo!("Implement task spawning logic");
    }
}
