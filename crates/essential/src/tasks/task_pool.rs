use async_channel::Sender;
use async_executor::FallibleTask;
use concurrent_queue::ConcurrentQueue;
use futures_lite::future::FutureExt;
use std::{future::Future, mem, num::NonZero, sync::Arc, thread::JoinHandle};

use crate::tasks::thread_executor::{Executor, ThreadExecutor, ThreadExecutorTicker};

use super::Task;

fn available_parallelism() -> usize {
    std::thread::available_parallelism()
        .map(NonZero::<usize>::get)
        .unwrap_or(1)
}

pub struct TaskPool {
    executor: Arc<Executor<'static>>,
    threads: Vec<JoinHandle<()>>,
    shutdown_send: Sender<()>,
}

impl TaskPool {
    thread_local! {
        static LOCAL_EXECUTOR: async_executor::LocalExecutor<'static> = const { async_executor::LocalExecutor::new() };
        static THREAD_EXECUTOR: Arc<ThreadExecutor<'static>> = Arc::new(ThreadExecutor::new());
    }

    pub fn new() -> Self {
        let executor = Arc::new(Executor::new());

        let (shutdown_send, shutdown_rcv) = async_channel::unbounded::<()>();

        let num_threads = available_parallelism();

        let threads = (0..num_threads)
            .map(|_| {
                let executor = Arc::clone(&executor);
                let shutdown_rcv = shutdown_rcv.clone();
                std::thread::Builder::new()
                    .spawn(move || {
                        Self::LOCAL_EXECUTOR.with(|local_executor| loop {
                            let res = std::panic::catch_unwind(|| {
                                let tick_forever = async move {
                                    loop {
                                        local_executor.tick().await;
                                    }
                                };
                                pollster::block_on(
                                    executor.run(tick_forever.or(shutdown_rcv.recv())),
                                )
                            });
                            if let Ok(val) = res {
                                val.unwrap_err();
                                break;
                            }
                        })
                    })
                    .expect("Failed to spawn thread")
            })
            .collect();

        TaskPool {
            executor,
            threads,
            shutdown_send,
        }
    }

    pub fn spawn<T>(&self, future: impl Future<Output = T> + Send + 'static) -> Task<T>
    where
        T: Send + 'static,
    {
        Task::new(self.executor.spawn(future))
    }

    pub fn spawn_local<T>(&self, future: impl Future<Output = T> + 'static) -> Task<T>
    where
        T: Send + 'static,
    {
        Task::new(Self::LOCAL_EXECUTOR.with(|local_executor| local_executor.spawn(future)))
    }

    pub fn scope<'env, F, T>(&self, f: F) -> Vec<T>
    where
        F: for<'scope> FnOnce(&'scope ScopedTaskPool<'scope, T>),
        T: Send + 'static,
    {
        Self::THREAD_EXECUTOR.with(|scope_executor| self.scope_inner(scope_executor, f))
    }

    fn scope_inner<'env, F, T>(&self, scope_executor: &ThreadExecutor, f: F) -> Vec<T>
    where
        F: for<'scope> FnOnce(&'scope ScopedTaskPool<'scope, T>),
        T: Send + 'static,
    {
        let executor: &Executor = &self.executor;
        let executor: &'env Executor = unsafe { mem::transmute(executor) };

        let spawned_tasks: ConcurrentQueue<FallibleTask<T>> = ConcurrentQueue::unbounded();
        let spawned_tasks: &'env ConcurrentQueue<FallibleTask<T>> =
            unsafe { mem::transmute(&spawned_tasks) };

        let scope = ScopedTaskPool {
            executor,
            spawned_tasks,
        };
        let scope: &'env ScopedTaskPool<'_, T> = unsafe { mem::transmute(&scope) };
        f(scope);

        if spawned_tasks.is_empty() {
            Vec::new()
        } else {
            pollster::block_on(async move {
                let get_results = async {
                    let mut results = Vec::with_capacity(spawned_tasks.len());
                    while let Ok(task) = spawned_tasks.pop() {
                        if let Some(result) = task.await {
                            results.push(result);
                        }
                    }

                    results
                };

                let scope_ticker = scope_executor.ticker().unwrap();


                Self::execute_scope(scope_ticker, get_results).await
            })
        }
    }

    async fn execute_scope<'scope, 'ticker, T>(
        scope_ticker: ThreadExecutorTicker<'scope, 'ticker>,
        get_results: impl Future<Output = Vec<T>>) -> Vec<T> {

        let execute_forever = async {
            loop {
                let tick_forever = async {
                    loop {
                        scope_ticker.tick().await;
                    }
                };

                tick_forever.await;
            }
        };
        get_results.or(execute_forever).await
    }
}

impl Drop for TaskPool {
    fn drop(&mut self) {
        let _ = self.shutdown_send.close();

        for handle in self.threads.drain(..) {
            handle.join().expect("Failed to join thread");
        }
    }
}

impl Default for TaskPool {
    fn default() -> Self {
        Self::new()
    }
}

pub struct ScopedTaskPool<'a, T: Send> {
    executor: &'a Executor<'a>,
    spawned_tasks: &'a ConcurrentQueue<FallibleTask<T>>,
}

impl<'a, T: Send> ScopedTaskPool<'a, T> {
    pub fn spawn<F: Future<Output = T> + Send + 'a>(&self, f: F) {
        let task = self.executor.spawn(f).fallible();

        self.spawned_tasks.push(task).unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    #[test]
    fn spawn_returns_result() {
        let pool = TaskPool::new();
        let task = pool.spawn(async { 42u32 });
        let result = pollster::block_on(task);
        assert_eq!(result, 42);
    }

    #[test]
    fn spawn_multiple_tasks_concurrently() {
        let pool = TaskPool::new();
        let counter = Arc::new(AtomicUsize::new(0));

        let tasks: Vec<_> = (0..16)
            .map(|_| {
                let counter = Arc::clone(&counter);
                pool.spawn(async move {
                    counter.fetch_add(1, Ordering::Relaxed);
                })
            })
            .collect();

        for task in tasks {
            pollster::block_on(task);
        }

        assert_eq!(counter.load(Ordering::Relaxed), 16);
    }

    #[test]
    fn scope_collects_results() {
        let pool = TaskPool::new();
        let results = pool.scope(|scope| {
            for i in 0u32..8 {
                scope.spawn(async move { i * 2 });
            }
        });

        assert_eq!(results.len(), 8);
        let mut sorted = results;
        sorted.sort();
        assert_eq!(sorted, vec![0, 2, 4, 6, 8, 10, 12, 14]);
    }

    #[test]
    fn scope_with_no_tasks_returns_empty() {
        let pool = TaskPool::new();
        let results = pool.scope(|_scope: &ScopedTaskPool<u32>| {});
        assert!(results.is_empty());
    }

    #[test]
    fn scope_shared_state_mutation() {
        let pool = TaskPool::new();
        let counter = Arc::new(AtomicUsize::new(0));

        pool.scope(|scope| {
            for _ in 0..4 {
                let counter = Arc::clone(&counter);
                scope.spawn(async move {
                    counter.fetch_add(1, Ordering::Relaxed);
                });
            }
        });

        assert_eq!(counter.load(Ordering::Relaxed), 4);
    }

    #[test]
    fn task_is_finished_after_await() {
        let pool = TaskPool::new();
        let task = pool.spawn(async { 1u32 });
        let result = pollster::block_on(task);
        assert_eq!(result, 1);
    }

    #[test]
    fn drop_joins_threads() {
        // If Drop impl is broken this will hang or panic.
        let pool = TaskPool::new();
        drop(pool);
    }
}
