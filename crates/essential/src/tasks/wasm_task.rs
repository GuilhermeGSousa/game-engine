use std::marker::PhantomData;

pub struct Task<T>(PhantomData<T>);

impl<T> Task<T> {
    pub fn new() -> Self {
        Task(PhantomData)
    }
}
