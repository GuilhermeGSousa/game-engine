use std::{any::TypeId, marker::PhantomData};

use typle::typle;

use crate::{
    component::{Component, ComponentId},
    entity::Entity,
    query_filter::QueryFilter,
    system::system_input::SystemInput,
    world::UnsafeWorldCell,
};

pub struct Query<'world, T: QueryData, F: QueryFilter = ()> {
    world: UnsafeWorldCell<'world>,
    matched_indices: Vec<usize>,
    _marker_data: PhantomData<T>,
    _marker_filter: PhantomData<F>,
}

pub trait QueryData {
    type Item<'a>;

    fn component_ids() -> Vec<ComponentId>;

    fn fetch<'w>(world: UnsafeWorldCell<'w>, entity: Entity) -> Option<Self::Item<'w>>;
}

impl<'world, T: QueryData, F: QueryFilter> Query<'world, T, F> {
    pub fn new(world: UnsafeWorldCell<'world>) -> Self {
        let matched_indices: Vec<usize> = world
            .get_world()
            .get_archetypes()
            .iter()
            .enumerate()
            .filter_map(|(index, archetype)| {
                if archetype.contains_all(T::component_ids()) {
                    Some(index)
                } else {
                    None
                }
            })
            .collect();

        Self {
            world,
            matched_indices,
            _marker_data: PhantomData,
            _marker_filter: PhantomData,
        }
    }

    pub fn iter<'s>(&'s self) -> QueryIter<'world, 's, T, F> {
        QueryIter {
            world: self.world,
            matched_archetypes: self.matched_indices.iter(),
            current_entities: &[],
            current_row: 0,
            current_len: 0,
            _marker_data: PhantomData,
            _marker_filter: PhantomData,
        }
    }

    pub fn get_entity(&self, entity: Entity) -> Option<T::Item<'world>> {
        T::fetch(self.world, entity)
    }
}

pub struct QueryIter<'world, 'a, T, F> {
    world: UnsafeWorldCell<'world>,
    matched_archetypes: core::slice::Iter<'a, usize>,
    current_entities: &'world [Entity],
    current_row: usize,
    current_len: usize,
    _marker_data: PhantomData<T>,
    _marker_filter: PhantomData<F>,
}

impl<'world, 's, T, F> Iterator for QueryIter<'world, 's, T, F>
where
    T: QueryData,
    F: QueryFilter,
{
    type Item = T::Item<'world>;

    fn next(&mut self) -> Option<Self::Item> {
        let archetypes = self.world.get_world().get_archetypes();
        loop {
            if self.current_row == self.current_len {
                let archetype_index = self.matched_archetypes.next()?;

                let archetype = &archetypes[*archetype_index];

                self.current_row = 0;
                self.current_len = archetype.len();
                self.current_entities = archetype.entities();
            }

            let entity = self.current_entities[self.current_row];
            self.current_row += 1;
            if !F::filter(self.world, entity) {
                continue;
            }

            return T::fetch(self.world, entity);
        }
    }
}

unsafe impl<T, F> SystemInput for Query<'_, T, F>
where
    T: QueryData,
    F: QueryFilter,
{
    type State = ();
    type Data<'world, 'state> = Query<'world, T, F>;

    fn init_state() -> Self::State {
        ()
    }

    unsafe fn get_data<'world, 'state>(
        _state: &'state mut Self::State,
        world: UnsafeWorldCell<'world>,
    ) -> Self::Data<'world, 'state> {
        Query::new(world)
    }
}

impl<T> QueryData for &T
where
    T: Component,
{
    type Item<'w> = &'w T;

    fn component_ids() -> Vec<ComponentId> {
        {
            vec![TypeId::of::<T>()]
        }
    }

    fn fetch<'w>(world: UnsafeWorldCell<'w>, entity: Entity) -> Option<Self::Item<'w>> {
        let world = world.get_world();

        if let Some(location) = world.get_entity_store().find_location(entity) {
            world.get_component_for_entity_location::<T>(location)
        } else {
            None
        }
    }
}

impl<T> QueryData for &mut T
where
    T: Component,
{
    type Item<'w> = &'w mut T;

    fn component_ids() -> Vec<ComponentId> {
        {
            vec![TypeId::of::<T>()]
        }
    }

    fn fetch<'w>(world: UnsafeWorldCell<'w>, entity: Entity) -> Option<Self::Item<'w>> {
        let world = world.get_world_mut();
        if let Some(location) = world.get_entity_store().find_location(entity) {
            world.get_component_for_entity_location_mut(location)
        } else {
            None
        }
    }
}

impl QueryData for Entity {
    type Item<'a> = Entity;

    fn component_ids() -> Vec<ComponentId> {
        vec![]
    }

    fn fetch<'w>(_world: UnsafeWorldCell<'w>, entity: Entity) -> Option<Self::Item<'w>> {
        Some(entity)
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

    fn component_ids() -> Vec<ComponentId> {
        {
            let mut res = Vec::new();

            for typle_index!(i) in 0..T::LEN {
                res.extend(T::<{ i }>::component_ids());
            }

            res
        }
    }

    fn fetch<'w>(world: UnsafeWorldCell<'w>, entity: Entity) -> Option<Self::Item<'w>> {
        Some(typle_for!(i in .. => <T<{i}>>::fetch(world, entity).unwrap()))
    }
}
