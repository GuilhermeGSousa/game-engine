pub mod task_pool;

pub struct Task<T> {
    _marker: std::marker::PhantomData<T>,
}
