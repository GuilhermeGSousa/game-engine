use std::{any::TypeId, marker::PhantomData};

use typle::typle;

use crate::{
    component::{Component, ComponentId},
    entity::{Entity, EntityLocation},
    system::system_input::SystemInput,
    table::TableRow,
    world::{self, UnsafeWorldCell},
};

pub struct Query<'world, T: QueryData> {
    world: UnsafeWorldCell<'world>,
    _marker: PhantomData<T>,
}

pub trait QueryData {
    type Item<'a>;

    fn get_component_ids() -> Vec<ComponentId>;

    fn fetch<'w>(world: UnsafeWorldCell<'w>, entity: EntityLocation) -> Option<Self::Item<'w>>;
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
        let entity_store = self.world.get_world().get_entity_store();

        if let Some(entity_location) = entity_store.find_location(entity) {
            T::fetch(self.world, entity_location)
        } else {
            None
        }
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
                let location = EntityLocation {
                    archetype_index: index.try_into().unwrap(),
                    row: TableRow::new(self.current_entity_index.try_into().unwrap()),
                };
                match T::fetch(self.world, location) {
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

    fn fetch<'w>(world: UnsafeWorldCell<'w>, location: EntityLocation) -> Option<Self::Item<'w>> {
        world
            .get_world()
            .get_component_for_entity_location::<T>(location)
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
        entity_location: EntityLocation,
    ) -> Option<Self::Item<'w>> {
        world
            .get_world_mut()
            .get_component_for_entity_location_mut(entity_location)
    }
}

impl QueryData for Entity {
    type Item<'a> = Entity;

    fn get_component_ids() -> Vec<ComponentId> {
        vec![]
    }

    fn fetch<'w>(
        world: UnsafeWorldCell<'w>,
        entity_location: EntityLocation,
    ) -> Option<Self::Item<'w>> {
        world
            .get_world()
            .get_entity_store()
            .find_entity_at_location(entity_location)
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
        entity_location: EntityLocation,
    ) -> Option<Self::Item<'w>> {
        Some(typle_for!(i in .. => <T<{i}>>::fetch(world, entity_location).unwrap()))
    }
}
