use async_channel::Sender;
use futures_lite::future::FutureExt;
use std::{future::Future, num::NonZero, sync::Arc, thread::JoinHandle};

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
                            let tick_forever = async move {
                                loop {
                                    local_executor.tick().await;
                                }
                            };
                            let res = pollster::block_on(
                                executor.run(tick_forever.or(shutdown_rcv.recv())),
                            );

                            res.unwrap_err();
                            break;
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
}

impl Drop for TaskPool {
    fn drop(&mut self) {
        let _ = self.shutdown_send.close();

        for handle in self.threads.drain(..) {
            handle.join().expect("Failed to join thread");
        }
    }
}
