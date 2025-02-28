use crate::{
    archetype::Archetype,
    component::{Component, ComponentId},
};

pub trait SystemInput {
    fn get_component_id() -> ComponentId;
    unsafe fn get_component_data_unsafe(archetype: &mut Archetype, index: usize) -> Self;
}

// Implement for immutable access (&T)
impl<'a, T: Component + 'static> SystemInput for &'a T {
    fn get_component_id() -> ComponentId {
        ComponentId::of::<T>()
    }

    unsafe fn get_component_data_unsafe(archetype: &mut Archetype, index: usize) -> Self {
        &*(archetype.get_component_unsafe(index) as *const T)
    }
}

// Implement for mutable access (&mut T)
impl<'a, T: Component + 'static> SystemInput for &'a mut T {
    fn get_component_id() -> ComponentId {
        ComponentId::of::<T>()
    }

    unsafe fn get_component_data_unsafe(archetype: &mut Archetype, index: usize) -> Self {
        &mut *(archetype.get_component_mut_unsafe(index) as *mut T)
    }
}
