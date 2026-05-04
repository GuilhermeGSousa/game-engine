use async_channel::Sender;
use async_executor::FallibleTask;
use concurrent_queue::ConcurrentQueue;
use futures_lite::future::FutureExt;
use pollster::block_on;
use std::{future::Future, num::NonZero, sync::Arc, thread::JoinHandle};

use crate::tasks::thread_executor::{Executor, ThreadExecutor};

use super::Task;

fn available_parallelism() -> usize {
    std::thread::available_parallelism()
        .map(NonZero::<usize>::get)
        .unwrap_or(1)
}

pub struct TaskPool {
    executor: Arc<async_executor::Executor<'static>>,
    threads: Vec<JoinHandle<()>>,
    shutdown_send: Sender<()>,
}

impl TaskPool {
    thread_local! {
        static LOCAL_EXECUTOR: async_executor::LocalExecutor<'static> = const { async_executor::LocalExecutor::new() };
        static THREAD_EXECUTOR: Arc<ThreadExecutor<'static>> = Arc::new(ThreadExecutor::new());
    }

    pub fn new() -> Self {
        let executor = Arc::new(async_executor::Executor::new());

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

    pub fn scope<F, T>(&self, f: F) -> Vec<T>
    where
        F: for<'scope> FnOnce(&'scope ScopedTaskPool<'scope, T>),
        T: Send + 'static,
    {
        let spawned_tasks: ConcurrentQueue<FallibleTask<T>> = ConcurrentQueue::unbounded();

        let scope = ScopedTaskPool {
            executor: todo!(),
            spawned_tasks: &spawned_tasks,
        };

        f(&scope);

        if spawned_tasks.is_empty() {
            Vec::new()
        } else {
            pollster::block_on(async move {
                let mut results = Vec::with_capacity(spawned_tasks.len());
                while let Ok(task) = spawned_tasks.pop() {
                    if let Some(result) = task.await {
                        results.push(result);
                    }
                }

                results
            })
        }
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
