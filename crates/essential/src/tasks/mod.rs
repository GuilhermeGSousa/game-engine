#[cfg_attr(target_arch = "wasm32", path = "wasm_task.rs")]
mod task;

pub use task::Task;

#[cfg_attr(target_arch = "wasm32", path = "wasm_task_pool.rs")]
pub mod task_pool;

#[cfg(test)]
mod tests {
    use super::task_pool::TaskPool;

    #[test]
    fn test_task() {
        let task_pool = TaskPool::new();
        let task_1 = task_pool.spawn(async {
            for i in 0..1000 {
                println!("Task 1 {}", i);
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        });

        let task_2 = task_pool.spawn(async {
            for i in 0..1000 {
                println!("Task 2 {}", i);
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        });

        //while !task_1.is_finished() || !task_2.is_finished() {}
    }
}
