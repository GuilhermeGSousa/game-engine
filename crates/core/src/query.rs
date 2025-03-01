use std::marker::PhantomData;

use crate::{bundle::ComponentBundle, system_input::SystemInput, world::UnsafeWorldCell};

pub struct Query<'world, T: ComponentBundle> {
    world: UnsafeWorldCell<'world>,
    _marker: PhantomData<T>,
}

impl<'world, T: ComponentBundle> Query<'world, T> {
    pub fn new(world: UnsafeWorldCell<'world>) -> Self {
        Self {
            world,
            _marker: PhantomData,
        }
    }

    pub fn iter(&self) -> QueryIter<'world, T> {
        QueryIter {
            world: self.world,
            _marker: PhantomData,
        }
    }
}

pub struct QueryIter<'world, T> {
    world: UnsafeWorldCell<'world>,
    _marker: PhantomData<T>,
}

impl<'world, T> Iterator for QueryIter<'world, T>
where
    T: ComponentBundle,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

unsafe impl<T> SystemInput for Query<'_, T>
where
    T: ComponentBundle,
{
    type Data<'world> = Query<'world, T>;

    unsafe fn get_data<'world>(world: UnsafeWorldCell<'world>) -> Self::Data<'world> {
        Query::new(world)
    }
}
