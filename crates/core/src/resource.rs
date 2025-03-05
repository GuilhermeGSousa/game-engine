use std::{
    any::{Any, TypeId},
    marker::PhantomData,
};

pub use core_macros::Resource;

use crate::{system_input::SystemInput, world::UnsafeWorldCell};

pub type ResourceId = TypeId;

pub trait ToAny: 'static {
    fn as_any(&self) -> &dyn Any;
    fn as_any_mut(&mut self) -> &mut dyn Any;
}

impl<T: 'static> ToAny for T {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }
}

pub trait Resource: 'static {
    fn name() -> String
    where
        Self: Sized;
}

pub struct Res<'world, T: Resource> {
    pub value: UnsafeWorldCell<'world>,
    _marker: PhantomData<T>,
}

impl<'world, T: Resource> Res<'world, T> {
    pub fn new(world: UnsafeWorldCell<'world>) -> Self {
        Self {
            value: world,
            _marker: PhantomData,
        }
    }
}

unsafe impl<T> SystemInput for Res<'_, T>
where
    T: Resource,
{
    type Data<'world> = Res<'world, T>;

    unsafe fn get_data<'world>(world: crate::world::UnsafeWorldCell<'world>) -> Self::Data<'world> {
        Res::new(world)
    }
}
