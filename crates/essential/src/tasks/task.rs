pub struct Task<T>(async_executor::Task<T>);

impl<T> Task<T> {
    pub fn new(task: async_executor::Task<T>) -> Self {
        Task(task)
    }

    pub fn is_finished(&self) -> bool {
        self.0.is_finished()
    }
}
