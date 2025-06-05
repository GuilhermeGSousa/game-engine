use std::{any::TypeId, marker::PhantomData};

use typle::typle;

use crate::{
    component::{Component, ComponentId},
    entity::Entity,
    system::system_input::SystemInput,
    world::UnsafeWorldCell,
};

pub struct Query<'world, T: QueryData> {
    world: UnsafeWorldCell<'world>,
    _marker: PhantomData<T>,
}

pub trait QueryData {
    type Item<'a>;

    fn get_component_ids() -> Vec<ComponentId>;

    fn fetch<'w>(
        world: UnsafeWorldCell<'w>,
        archetype_index: usize,
        entity_index: usize,
    ) -> Option<Self::Item<'w>>;
}

enum ArchetypeIteratorState {
    Pending(usize),
    Iterating(usize),
}

impl<'world, T: QueryData> Query<'world, T> {
    pub fn new(world: UnsafeWorldCell<'world>) -> Self {
        Self {
            world,
            _marker: PhantomData,
        }
    }

    pub fn iter(&self) -> QueryIter<'world, T> {
        QueryIter {
            world: self.world,
            archetype_iteration_state: ArchetypeIteratorState::Pending(0),
            current_entity_index: 0,
            _marker: PhantomData,
        }
    }

    pub fn get_entity(&self, entity: Entity) -> Option<T::Item<'world>> {
        None
    }
}

pub struct QueryIter<'world, T> {
    world: UnsafeWorldCell<'world>,
    archetype_iteration_state: ArchetypeIteratorState,
    current_entity_index: usize,
    _marker: PhantomData<T>,
}

impl<'world, T> Iterator for QueryIter<'world, T>
where
    T: QueryData,
{
    type Item = T::Item<'world>;

    fn next(&mut self) -> Option<Self::Item> {
        let archetypes = self.world.get_world_mut().get_archetypes_mut();

        if let ArchetypeIteratorState::Pending(pending_index) = self.archetype_iteration_state {
            // Loop until we find a good one
            let mut current_index = pending_index;
            while current_index < archetypes.len() {
                if archetypes[current_index].contains_all(T::get_component_ids()) {
                    self.archetype_iteration_state =
                        ArchetypeIteratorState::Iterating(current_index);
                    self.current_entity_index = 0;
                    break;
                }

                current_index += 1;
            }
        }

        match self.archetype_iteration_state {
            ArchetypeIteratorState::Pending(_) => None,
            ArchetypeIteratorState::Iterating(index) => {
                match T::fetch(self.world, index, self.current_entity_index) {
                    Some(value) => {
                        self.current_entity_index += 1;
                        if self.current_entity_index >= archetypes[index].len() {
                            self.archetype_iteration_state =
                                ArchetypeIteratorState::Pending(index + 1);
                        }

                        return Some(value);
                    }
                    None => {
                        panic!("This should not be possible!")
                    }
                };
            }
        }
    }
}

unsafe impl<T> SystemInput for Query<'_, T>
where
    T: QueryData,
{
    type Data<'world> = Query<'world, T>;

    unsafe fn get_data<'world>(world: UnsafeWorldCell<'world>) -> Self::Data<'world> {
        Query::new(world)
    }
}

impl<T> QueryData for &T
where
    T: Component,
{
    type Item<'w> = &'w T;

    fn get_component_ids() -> Vec<ComponentId> {
        {
            vec![TypeId::of::<T>()]
        }
    }

    fn fetch<'w>(
        world: UnsafeWorldCell<'w>,
        archetype_index: usize,
        entity_index: usize,
    ) -> Option<Self::Item<'w>> {
        let archetype = &world.get_world().get_archetypes()[archetype_index];
        unsafe { Some(archetype.get_component_unsafe(entity_index)) }
    }
}

impl<T> QueryData for &mut T
where
    T: Component,
{
    type Item<'w> = &'w mut T;

    fn get_component_ids() -> Vec<ComponentId> {
        {
            vec![TypeId::of::<T>()]
        }
    }

    fn fetch<'w>(
        world: UnsafeWorldCell<'w>,
        archetype_index: usize,
        entity_index: usize,
    ) -> Option<Self::Item<'w>> {
        let archetype = &mut world.get_world_mut().get_archetypes_mut()[archetype_index];
        unsafe { Some(archetype.get_component_mut_unsafe(entity_index)) }
    }
}

#[allow(unused_mut)]
#[allow(unused_variables)]
#[typle(Tuple for 0..=12)]
impl<T> QueryData for T
where
    T: Tuple,
    T<_>: QueryData,
{
    type Item<'w> = typle_for!(i in .. => T<{i}>::Item<'w>);

    fn get_component_ids() -> Vec<ComponentId> {
        {
            let mut res = Vec::new();

            for typle_index!(i) in 0..T::LEN {
                res.extend(T::<{ i }>::get_component_ids());
            }

            res
        }
    }

    fn fetch<'w>(
        world: UnsafeWorldCell<'w>,
        archetype_index: usize,
        entity_index: usize,
    ) -> Option<Self::Item<'w>> {
        Some(typle_for!(i in .. => <T<{i}>>::fetch(world, archetype_index, entity_index).unwrap()))
    }
}
