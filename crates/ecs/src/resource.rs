use std::{
    any::TypeId,
    marker::PhantomData,
    ops::{Deref, DerefMut},
};

pub use ecs_macros::Resource;

use crate::{system::system_input::SystemInput, world::UnsafeWorldCell};

pub type ResourceId = TypeId;

pub trait Resource: 'static {
    fn name() -> String
    where
        Self: Sized;
}

pub struct Res<'world, T: Resource> {
    pub value: &'world T,
    _marker: PhantomData<T>,
}

impl<'world, T: Resource> Res<'world, T> {
    pub fn new(world: UnsafeWorldCell<'world>) -> Self {
        Self {
            value: world.get_world().get_resource::<T>().unwrap(),
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

impl<T> Deref for Res<'_, T>
where
    T: Resource,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

pub struct ResMut<'world, T: Resource> {
    pub value: &'world mut T,
    _marker: PhantomData<T>,
}

impl<'world, T: Resource> ResMut<'world, T> {
    pub fn new(world: UnsafeWorldCell<'world>) -> Self {
        Self {
            value: world.get_world_mut().get_resource_mut::<T>().unwrap(),
            _marker: PhantomData,
        }
    }
}

unsafe impl<T> SystemInput for ResMut<'_, T>
where
    T: Resource,
{
    type Data<'world> = ResMut<'world, T>;

    unsafe fn get_data<'world>(world: crate::world::UnsafeWorldCell<'world>) -> Self::Data<'world> {
        ResMut::new(world)
    }
}

impl<T> Deref for ResMut<'_, T>
where
    T: Resource,
{
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.value
    }
}

impl<T> DerefMut for ResMut<'_, T>
where
    T: Resource,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.value
    }
}
