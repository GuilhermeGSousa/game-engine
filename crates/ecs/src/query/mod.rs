use std::{any::TypeId, marker::PhantomData};

use typle::typle;

pub mod change_detection;
pub mod query_filter;

use crate::{
    component::{Component, ComponentId, Tick},
    entity::Entity,
    query::{change_detection::Mut, query_filter::QueryFilter},
    system::{access::SystemAccess, input::SystemInput},
    table::{Column, TableRowIndex},
    world::{UnsafeWorldCell, World},
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

/// Cached, per-system state for a [`Query`].
///
/// Matching every archetype in the world on every run is wasteful: archetypes
/// are only ever appended (never removed or reordered), so once an archetype has
/// been classified as matching or not, that verdict never changes. `QueryState`
/// remembers how many archetypes it has already inspected and only classifies
/// the newly-appended ones on subsequent runs, making steady-state query setup
/// `O(new archetypes)` instead of `O(all archetypes)`.
pub struct QueryState {
    matched_indices: Vec<usize>,
    archetypes_len: usize,
}

impl QueryState {
    pub fn new() -> Self {
        Self {
            matched_indices: Vec::new(),
            archetypes_len: 0,
        }
    }

    /// Classifies any archetypes appended since the last call, appending the
    /// indices of new matches to `matched_indices`.
    fn update<T: QueryData, F: QueryFilter>(&mut self, world: &World) {
        let archetypes = world.archetypes();
        if self.archetypes_len == archetypes.len() {
            return;
        }

        // Only reached when new archetypes have appeared; the steady-state
        // early-return above keeps this zone out of most frames.
        profiling::scope!("query::match_archetypes");

        // Compute the required component ids once, not once per archetype.
        let required = T::component_ids();
        let start = self.archetypes_len;
        for (offset, archetype) in archetypes[start..].iter().enumerate() {
            if archetype.contains_all(&required) && F::matches_archetype(archetype) {
                self.matched_indices.push(start + offset);
            }
        }
        self.archetypes_len = archetypes.len();
    }
}

impl Default for QueryState {
    fn default() -> Self {
        Self::new()
    }
}

/// Describes what data a [`Query`] fetches from each matching entity.
///
/// Implementations are provided for `&T`, `&mut T`, `Entity`, `Option<&T>`,
/// `Option<&mut T>`, and tuples of up to 12 elements.
pub trait QueryData {
    /// The item type produced for each matched entity.
    type Item<'a>;

    /// Per-archetype state that resolves this query's component columns **once**
    /// when an archetype is entered, so that fetching a row does not repeat the
    /// component-id hash lookup for every entity.
    type Fetch<'w>;

    /// Returns the [`ComponentId`]s that must be present on an entity for it to match.
    fn component_ids() -> Vec<ComponentId>;

    /// Resolves the columns for `archetype_index` a single time. Called once per
    /// matched archetype by the iterator.
    fn init_fetch<'w>(world: UnsafeWorldCell<'w>, archetype_index: usize) -> Self::Fetch<'w>;

    /// Fetches the data for the entity at `row`, using the pre-resolved [`Fetch`](Self::Fetch).
    fn fetch<'w>(
        fetch: &Self::Fetch<'w>,
        entity: Entity,
        row: TableRowIndex,
    ) -> Option<Self::Item<'w>>;

    /// Registers component access with the scheduler's access tracker.
    fn fill_access(access: &mut SystemAccess);
}

impl<'world, T: QueryData, F: QueryFilter> Query<'world, T, F> {
    /// Constructs a new query by scanning **all** of the world's archetypes for
    /// matches.
    ///
    /// Systems go through the cached [`QueryState`] path instead (see the
    /// [`SystemInput`] implementation); this full scan is primarily for
    /// standalone / ad-hoc use against a world.
    pub fn new(world: UnsafeWorldCell<'world>) -> Self {
        profiling::scope!("query::scan", std::any::type_name::<T>());
        let required = T::component_ids();
        let matched_indices: Vec<usize> = world
            .world()
            .archetypes()
            .iter()
            .enumerate()
            .filter_map(|(index, archetype)| {
                if archetype.contains_all(&required) && F::matches_archetype(archetype) {
                    Some(index)
                } else {
                    None
                }
            })
            .collect();

        Self::from_matched_indices(world, matched_indices)
    }

    fn from_matched_indices(world: UnsafeWorldCell<'world>, matched_indices: Vec<usize>) -> Self {
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
            current_fetch: None,
            current_entities: &[],
            current_row: 0,
            current_len: 0,
            // Presence/absence filters are decided per archetype, so the
            // per-entity filter can be skipped entirely for them.
            skip_entity_filter: F::is_archetypal(),
            _marker_filter: PhantomData,
        }
    }

    /// Fetches the query data for a specific entity, returning `None` if it doesn't match.
    pub fn get_entity(&self, entity: Entity) -> Option<T::Item<'world>> {
        if !F::filter(self.world, entity) {
            return None;
        }
        let location = self.world.world().entity_store().find_location(entity)?;
        let fetch = T::init_fetch(self.world, location.archetype_index as usize);
        T::fetch(&fetch, entity, location.row)
    }

    /// Returns `true` if `entity` matches this query.
    pub fn contains_entity(&self, entity: Entity) -> bool {
        self.get_entity(entity).is_some()
    }
}

pub struct QueryIter<'world, 's, T: QueryData, F> {
    world: UnsafeWorldCell<'world>,
    matched_archetypes: core::slice::Iter<'s, usize>,
    current_fetch: Option<T::Fetch<'world>>,
    current_entities: &'world [Entity],
    current_row: usize,
    current_len: usize,
    skip_entity_filter: bool,
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
                let archetype_index = *self.matched_archetypes.next()?;

                let archetype = &archetypes[archetype_index];

                self.current_row = 0;
                self.current_len = archetype.len();
                self.current_entities = archetype.entities();
                // Resolve the component columns for this archetype exactly once.
                self.current_fetch = Some(T::init_fetch(self.world, archetype_index));
                continue;
            }

            let row = self.current_row;
            let entity = self.current_entities[row];
            self.current_row += 1;

            if !self.skip_entity_filter && !F::filter(self.world, entity) {
                continue;
            }

            let fetch = self
                .current_fetch
                .as_ref()
                .expect("fetch is initialized when current_len > 0");
            if let Some(item) = T::fetch(fetch, entity, TableRowIndex::new(row)) {
                return Some(item);
            }
        }
    }
}

impl<T, F> SystemInput for Query<'_, T, F>
where
    T: QueryData,
    F: QueryFilter,
{
    type State = QueryState;
    type Data<'world, 'state> = Query<'world, T, F>;

    fn init_state() -> Self::State {
        QueryState::new()
    }

    fn get_data<'world, 'state>(
        state: &'state mut Self::State,
        world: UnsafeWorldCell<'world>,
    ) -> Self::Data<'world, 'state> {
        profiling::scope!("query::fetch_state", std::any::type_name::<T>());
        state.update::<T, F>(world.world());
        Query::from_matched_indices(world, state.matched_indices.clone())
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
    type Fetch<'w> = Option<&'w Column>;

    fn component_ids() -> Vec<ComponentId> {
        {
            vec![TypeId::of::<T>()]
        }
    }

    fn init_fetch<'w>(world: UnsafeWorldCell<'w>, archetype_index: usize) -> Self::Fetch<'w> {
        world
            .world()
            .archetypes()
            .get(archetype_index)
            .and_then(|archetype| archetype.get_column(ComponentId::of::<T>()))
    }

    fn fetch<'w>(
        fetch: &Self::Fetch<'w>,
        _entity: Entity,
        row: TableRowIndex,
    ) -> Option<Self::Item<'w>> {
        (*fetch).and_then(|column| unsafe { column.get_unsafe::<T>(row) })
    }

    fn fill_access(access: &mut SystemAccess) {
        access.read_component::<T>();
    }
}

/// Per-archetype fetch state for mutable component access.
///
/// Holds a raw pointer to the resolved [`Column`] (resolved once when the
/// archetype is entered) plus the current tick, so each row can be fetched
/// without re-hashing the component id.
pub struct WriteFetch<'w> {
    column: Option<*mut Column>,
    current_tick: Tick,
    _marker: PhantomData<&'w mut Column>,
}

impl<'w> WriteFetch<'w> {
    fn init<T: Component>(world: UnsafeWorldCell<'w>, archetype_index: usize) -> Self {
        let world = world.world_mut();
        let current_tick = world.current_tick();
        let column = world
            .get_archetypes_mut()
            .get_mut(archetype_index)
            .and_then(|archetype| archetype.get_column_mut(ComponentId::of::<T>()))
            .map(|column| column as *mut Column);

        Self {
            column,
            current_tick,
            _marker: PhantomData,
        }
    }

    fn fetch_row<T: Component>(&self, row: TableRowIndex) -> Option<Mut<'w, T>> {
        let column_ptr = self.column?;
        // SAFETY: the pointer was derived from a `'w` mutable world borrow and
        // the scheduler's access tracker guarantees exclusive access to this
        // component for the duration of `'w`. Each row is visited at most once,
        // so the produced references never alias.
        let column: &'w mut Column = unsafe { &mut *column_ptr };
        unsafe {
            column
                .get_unsafe_mut::<T>(row)
                .map(|accessor| Mut::new(accessor.data, accessor.changed_tick, self.current_tick))
        }
    }
}

impl<T> QueryData for &mut T
where
    T: Component,
{
    type Item<'w> = Mut<'w, T>;
    type Fetch<'w> = WriteFetch<'w>;

    fn component_ids() -> Vec<ComponentId> {
        {
            vec![TypeId::of::<T>()]
        }
    }

    fn init_fetch<'w>(world: UnsafeWorldCell<'w>, archetype_index: usize) -> Self::Fetch<'w> {
        WriteFetch::init::<T>(world, archetype_index)
    }

    fn fetch<'w>(
        fetch: &Self::Fetch<'w>,
        _entity: Entity,
        row: TableRowIndex,
    ) -> Option<Self::Item<'w>> {
        fetch.fetch_row::<T>(row)
    }

    fn fill_access(access: &mut SystemAccess) {
        access.write_component::<T>();
    }
}

impl QueryData for Entity {
    type Item<'a> = Entity;
    type Fetch<'w> = ();

    fn component_ids() -> Vec<ComponentId> {
        vec![]
    }

    fn init_fetch<'w>(_world: UnsafeWorldCell<'w>, _archetype_index: usize) -> Self::Fetch<'w> {}

    fn fetch<'w>(
        _fetch: &Self::Fetch<'w>,
        entity: Entity,
        _row: TableRowIndex,
    ) -> Option<Self::Item<'w>> {
        Some(entity)
    }

    fn fill_access(_access: &mut SystemAccess) {}
}

impl<T> QueryData for Option<&T>
where
    T: Component,
{
    type Item<'w> = Option<&'w T>;
    type Fetch<'w> = Option<&'w Column>;

    fn component_ids() -> Vec<ComponentId> {
        vec![]
    }

    fn init_fetch<'w>(world: UnsafeWorldCell<'w>, archetype_index: usize) -> Self::Fetch<'w> {
        world
            .world()
            .archetypes()
            .get(archetype_index)
            .and_then(|archetype| archetype.get_column(ComponentId::of::<T>()))
    }

    fn fetch<'w>(
        fetch: &Self::Fetch<'w>,
        _entity: Entity,
        row: TableRowIndex,
    ) -> Option<Self::Item<'w>> {
        Some((*fetch).and_then(|column| unsafe { column.get_unsafe::<T>(row) }))
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
    type Fetch<'w> = WriteFetch<'w>;

    fn component_ids() -> Vec<ComponentId> {
        vec![]
    }

    fn init_fetch<'w>(world: UnsafeWorldCell<'w>, archetype_index: usize) -> Self::Fetch<'w> {
        WriteFetch::init::<T>(world, archetype_index)
    }

    fn fetch<'w>(
        fetch: &Self::Fetch<'w>,
        _entity: Entity,
        row: TableRowIndex,
    ) -> Option<Self::Item<'w>> {
        Some(fetch.fetch_row::<T>(row))
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
    type Fetch<'w> = typle_for!(i in .. => T<{i}>::Fetch<'w>);

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

    fn init_fetch<'w>(world: UnsafeWorldCell<'w>, archetype_index: usize) -> Self::Fetch<'w> {
        typle_for!(i in .. => <T<{i}>>::init_fetch(world, archetype_index))
    }

    fn fetch<'w>(
        fetch: &Self::Fetch<'w>,
        entity: Entity,
        row: TableRowIndex,
    ) -> Option<Self::Item<'w>> {
        Some(typle_for!(i in .. => {
                <T<{i}>>::fetch(&fetch[[i]], entity, row)?
            }
        ))
    }

    fn fill_access(access: &mut SystemAccess) {
        for typle_index!(i) in 0..T::LEN {
            <T<{ i }>>::fill_access(access);
        }
    }
}
