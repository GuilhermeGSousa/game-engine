use std::any::{Any, TypeId};

pub use core_macros::Resource;

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
    pub value: &'world T,
}

// unsafe impl<'world, T: Resource> SystemInput for Res<'world, T> {
//     unsafe fn get_data(world: UnsafeWorldCell<'world>) -> Self {
//         Self {
//             value: world.get_world().get_resource().unwrap(),
//         }
//     }
// }

// unsafe impl<'world, T: Resource> SystemInput for Option<Res<'world, T>> {
//     unsafe fn get_data(world: UnsafeWorldCell<'world>) -> Self {
//         if let Some(resource) = world.get_world().get_resource() {
//             Some(Res { value: resource })
//         } else {
//             None
//         }
//     }
// }

pub struct ResMut<'a, T: Resource> {
    pub value: &'a mut T,
}

// unsafe impl<'world, T: Resource> SystemInput for ResMut<'world, T> {
//     unsafe fn get_data(world: UnsafeWorldCell<'world>) -> Self {
//         Self {
//             value: world.get_world_mut().get_resource_mut().unwrap(),
//         }
//     }
// }

// unsafe impl<'world, T: Resource> SystemInput for Option<ResMut<'world, T>> {
//     unsafe fn get_data(world: UnsafeWorldCell<'world>) -> Self {
//         if let Some(resource) = world.get_world_mut().get_resource_mut() {
//             Some(ResMut { value: resource })
//         } else {
//             None
//         }
//     }
// }
