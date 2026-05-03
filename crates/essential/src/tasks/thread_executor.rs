use std::thread::{self, ThreadId};

use derive_more::{Deref, DerefMut};

type ExecutorInner<'a> = async_executor::Executor<'a>;
type LocalExecutorInner<'a> = async_executor::LocalExecutor<'a>;

#[derive(Deref, DerefMut, Default)]
pub struct Executor<'a>(ExecutorInner<'a>);

impl<'a> Executor<'a> {
    pub const fn new() -> Self {
        Self(ExecutorInner::new())
    }
}

pub struct ThreadExecutor<'task> {
    executor: Executor<'task>,
    thread_id: ThreadId,
}

impl<'task> ThreadExecutor<'task> {
    pub fn new() -> Self {
        Self {
            executor: Executor::new(),
            thread_id: thread::current().id(),
        }
    }
}
