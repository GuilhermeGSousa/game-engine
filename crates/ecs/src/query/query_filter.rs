use std::marker::PhantomData;

use crate::{component::bundle::ComponentBundle, entity::Entity, world::UnsafeWorldCell};
use typle::typle;

/// Restricts which entities a [`Query`](super::Query) visits.
///
/// Multiple filters can be combined in a tuple: `(With<A>, Without<B>)` matches
/// entities that have `A` but not `B`.  [`Or`] can be used for disjunctions.
pub trait QueryFilter {
    fn filter<'w>(world: UnsafeWorldCell<'w>, entity: Entity) -> bool {
        Self::filter_and(world, entity)
    }

    fn filter_and<'w>(_world: UnsafeWorldCell<'w>, _entity: Entity) -> bool {
        true
    }

    fn filter_or<'w>(_world: UnsafeWorldCell<'w>, _entity: Entity) -> bool {
        true
    }
}

#[allow(unused_mut)]
#[allow(unused_variables)]
#[typle(Tuple for 0..=12)]
impl<T> QueryFilter for T
where
    T: Tuple,
    T<_>: QueryFilter,
{
    fn filter_and<'w>(world: UnsafeWorldCell<'w>, entity: Entity) -> bool {
        for typle_index!(i) in 0..T::LEN {
            if !T::<{ i }>::filter(world, entity) {
                return false;
            }
        }
        true
    }

    fn filter_or<'w>(world: UnsafeWorldCell<'w>, entity: Entity) -> bool {
        for typle_index!(i) in 0..T::LEN {
            if T::<{ i }>::filter_or(world, entity) {
                return true;
            }
        }
        false
    }
}

/// Matches entities where the given components were **added** this tick.
pub struct Added<T: ComponentBundle> {
    _marker: PhantomData<T>,
}

impl<T> QueryFilter for Added<T>
where
    T: ComponentBundle,
{
    fn filter<'w>(world: UnsafeWorldCell<'w>, entity: Entity) -> bool {
        for component_id in T::get_component_ids() {
            if !world.world().was_component_added(entity, component_id) {
                return false;
            }
        }
        true
    }
}

/// Matches entities where the given components were **mutated** this tick.
pub struct Changed<T: ComponentBundle> {
    _marker: PhantomData<T>,
}

impl<T> QueryFilter for Changed<T>
where
    T: ComponentBundle,
{
    fn filter<'w>(world: UnsafeWorldCell<'w>, entity: Entity) -> bool {
        for component_id in T::get_component_ids() {
            if !world.world().was_component_changed(entity, component_id) {
                return false;
            }
        }
        true
    }
}

/// Matches entities that **have** all of the given components, without fetching them.
pub struct With<T: ComponentBundle> {
    _marker: PhantomData<T>,
}

impl<T> QueryFilter for With<T>
where
    T: ComponentBundle,
{
    fn filter<'w>(world: UnsafeWorldCell<'w>, entity: Entity) -> bool {
        let world = world.world();

        match world.entity_store().find_location(entity) {
            Some(location) => {
                let archetypes = world.archetypes();
                let archetype = &archetypes[location.archetype_index as usize];
                let archetype_ids = archetype.component_ids();
                let component_ids = T::get_component_ids();

                component_ids.iter().all(|id| archetype_ids.contains(id))
            }
            None => false,
        }
    }
}

/// Inverts a [`QueryFilter`]: matches entities that do **not** satisfy `T`.
pub struct Not<T: QueryFilter> {
    _marker: PhantomData<T>,
}

impl<T> QueryFilter for Not<T>
where
    T: QueryFilter,
{
    fn filter<'w>(world: UnsafeWorldCell<'w>, entity: Entity) -> bool {
        !T::filter(world, entity)
    }
}

/// Matches entities that **do not** have the given component(s). Alias for `Not<With<T>>`.
pub type Without<T> = Not<With<T>>;

/// Matches entities that satisfy **at least one** of the filters in the tuple `T`.
///
/// # Example
/// ```ignore
/// // Entities that have either Health or Shield
/// Query<Entity, Or<(With<Health>, With<Shield>)>>
/// ```
pub struct Or<T: QueryFilter> {
    _marker: PhantomData<T>,
}

impl<T> QueryFilter for Or<T>
where
    T: QueryFilter,
{
    fn filter<'w>(world: UnsafeWorldCell<'w>, entity: Entity) -> bool {
        T::filter_or(world, entity)
    }
}
