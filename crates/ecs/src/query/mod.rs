use std::{any::TypeId, marker::PhantomData};

use typle::typle;

pub mod change_detection;
pub mod query_filter;

use crate::{
    component::{Component, ComponentId},
    entity::Entity,
    query::{change_detection::Mut, query_filter::QueryFilter},
    system::{access::SystemAccess, system_input::SystemInput},
    world::UnsafeWorldCell,
};

/// A type-safe view over all entities in a [`World`](crate::world::World) that
/// match a given set of components.
///
/// `T` is a [`QueryData`] that describes which components to fetch.  `F` is an
/// optional [`QueryFilter`] that further restricts which entities are visited.
///
/// Obtain a `Query` in a system by adding it as a parameter:
///
/// ```ignore
/// fn move_system(query: Query<(&mut Transform, &Velocity)>) {
///     for (transform, velocity) in query.iter() {
///         transform.translation += velocity.linear;
///     }
/// }
/// ```
pub struct Query<'world, T: QueryData, F: QueryFilter = ()> {
    world: UnsafeWorldCell<'world>,
    matched_indices: Vec<usize>,
    _marker_data: PhantomData<T>,
    _marker_filter: PhantomData<F>,
}

/// Describes what data a [`Query`] fetches from each matching entity.
///
/// Implementations are provided for `&T`, `&mut T`, `Entity`, `Option<&T>`,
/// `Option<&mut T>`, and tuples of up to 12 elements.
pub trait QueryData {
    /// The item type produced for each matched entity.
    type Item<'a>;

    /// Returns the [`ComponentId`]s that must be present on an entity for it to match.
    fn component_ids() -> Vec<ComponentId>;

    /// Fetches the data for `entity` from `world`, returning `None` if not possible.
    fn fetch<'w>(world: UnsafeWorldCell<'w>, entity: Entity) -> Option<Self::Item<'w>>;

    /// Registers component access with the scheduler's access tracker.
    fn fill_access(access: &mut SystemAccess);
}

impl<'world, T: QueryData, F: QueryFilter> Query<'world, T, F> {
    /// Constructs a new query by scanning the world's archetypes for matches.
    pub fn new(world: UnsafeWorldCell<'world>) -> Self {
        let matched_indices: Vec<usize> = world
            .world()
            .archetypes()
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

    /// Returns an iterator over all matching entities.
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

    /// Fetches the query data for a specific entity, returning `None` if it doesn't match.
    pub fn get_entity(&self, entity: Entity) -> Option<T::Item<'world>> {
        if !F::filter(self.world, entity) {
            return None;
        }
        T::fetch(self.world, entity)
    }

    /// Returns `true` if `entity` matches this query.
    pub fn contains_entity(&self, entity: Entity) -> bool {
        self.get_entity(entity).is_some()
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
        let archetypes = self.world.world().archetypes();
        loop {
            if self.current_row == self.current_len {
                let archetype_index = self.matched_archetypes.next()?;

                let archetype = &archetypes[*archetype_index];

                self.current_row = 0;
                self.current_len = archetype.len();
                self.current_entities = archetype.entities();
            }

            if self.current_entities.is_empty() {
                continue;
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

impl<T, F> SystemInput for Query<'_, T, F>
where
    T: QueryData,
    F: QueryFilter,
{
    type State = ();
    type Data<'world, 'state> = Query<'world, T, F>;

    fn init_state() -> Self::State {}

    fn get_data<'world, 'state>(
        _state: &'state mut Self::State,
        world: UnsafeWorldCell<'world>,
    ) -> Self::Data<'world, 'state> {
        Query::new(world)
    }

    fn fill_access(access: &mut crate::system::access::SystemAccess) {
        T::fill_access(access);
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
        let world = world.world();
        world
            .entity_store()
            .find_location(entity)
            .and_then(|location| world.get_component_for_entity_location::<T>(location))
    }

    fn fill_access(access: &mut SystemAccess) {
        access.read_component::<T>();
    }
}

impl<T> QueryData for &mut T
where
    T: Component,
{
    type Item<'w> = Mut<'w, T>;

    fn component_ids() -> Vec<ComponentId> {
        {
            vec![TypeId::of::<T>()]
        }
    }

    fn fetch<'w>(world: UnsafeWorldCell<'w>, entity: Entity) -> Option<Self::Item<'w>> {
        let world = world.world_mut();

        world
            .entity_store()
            .find_location(entity)
            .and_then(|location| {
                let current_tick = world.current_tick();
                world
                    .get_component_for_entity_location_mut::<T>(location)
                    .map(|table_cell| {
                        Mut::new(table_cell.data, table_cell.changed_tick, current_tick)
                    })
            })
    }

    fn fill_access(access: &mut SystemAccess) {
        access.write_component::<T>();
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

    fn fill_access(_access: &mut SystemAccess) {}
}

impl<T> QueryData for Option<&T>
where
    T: Component,
{
    type Item<'w> = Option<&'w T>;

    fn component_ids() -> Vec<ComponentId> {
        vec![]
    }

    fn fetch<'w>(world: UnsafeWorldCell<'w>, entity: Entity) -> Option<Self::Item<'w>> {
        let world = world.world();
        world
            .entity_store()
            .find_location(entity)
            .map(|location| world.get_component_for_entity_location(location))
    }

    fn fill_access(access: &mut SystemAccess) {
        access.read_component::<T>();
    }
}

impl<T> QueryData for Option<&mut T>
where
    T: Component,
{
    type Item<'w> = Option<Mut<'w, T>>;

    fn component_ids() -> Vec<ComponentId> {
        vec![]
    }

    fn fetch<'w>(world: UnsafeWorldCell<'w>, entity: Entity) -> Option<Self::Item<'w>> {
        let world = world.world_mut();

        world.entity_store().find_location(entity).map(|location| {
            let current_tick = world.current_tick();
            world
                .get_component_for_entity_location_mut::<T>(location)
                .map(|table_cell| Mut::new(table_cell.data, table_cell.changed_tick, current_tick))
        })
    }

    fn fill_access(access: &mut SystemAccess) {
        access.write_component::<T>();
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

    #[allow(clippy::let_and_return)]
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
        Some(typle_for!(i in .. => {
                <T<{i}>>::fetch(world, entity)?
            }
        ))
    }

    fn fill_access(access: &mut SystemAccess) {
        for typle_index!(i) in 0..T::LEN {
            <T<{ i }>>::fill_access(access);
        }
    }
}
