use std::marker::PhantomData;

use crate::{
    archetype::Archetype, component::bundle::ComponentBundle, entity::Entity,
    world::UnsafeWorldCell,
};
use typle::typle;

/// Restricts which entities a [`Query`](super::Query) visits.
///
/// Multiple filters can be combined in a tuple: `(With<A>, Without<B>)` matches
/// entities that have `A` but not `B`.  [`Or`] can be used for disjunctions.
///
/// # Two levels of filtering
///
/// Filtering happens at two levels for performance:
///
/// * **Archetype-level** ([`matches_archetype`](QueryFilter::matches_archetype)):
///   whether a whole archetype can be skipped. Because every entity in an
///   archetype shares the same set of components, presence/absence filters like
///   [`With`]/[`Without`] are decided *once per archetype* at query-construction
///   time and never re-evaluated per entity.
/// * **Per-entity** ([`filter`](QueryFilter::filter)): row-level checks that
///   genuinely differ between entities in the same archetype, such as
///   [`Added`]/[`Changed`] change detection.
///
/// A filter reports via [`is_archetypal`](QueryFilter::is_archetypal) whether it
/// is fully decided at the archetype level. When it is, the iterator skips the
/// per-entity [`filter`](QueryFilter::filter) call entirely.
pub trait QueryFilter {
    /// Archetype-level pruning. Returns `false` only when **no** entity in
    /// `archetype` can possibly match, so it is always safe to skip that
    /// archetype. Conservative: returns `true` when unsure.
    fn matches_archetype(_archetype: &Archetype) -> bool {
        Self::matches_archetype_and(_archetype)
    }

    /// Whether [`matches_archetype`](QueryFilter::matches_archetype) is *exact* —
    /// i.e. every entity in a matched archetype is guaranteed to pass. When
    /// `true`, the iterator can skip the per-entity [`filter`](QueryFilter::filter).
    fn is_archetypal() -> bool {
        false
    }

    /// Conjunction (`AND`) of archetype-level matches across a tuple of filters.
    fn matches_archetype_and(_archetype: &Archetype) -> bool {
        true
    }

    /// Disjunction (`OR`) of archetype-level matches across a tuple of filters.
    fn matches_archetype_or(_archetype: &Archetype) -> bool {
        false
    }

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
    fn is_archetypal() -> bool {
        for typle_index!(i) in 0..T::LEN {
            if !T::<{ i }>::is_archetypal() {
                return false;
            }
        }
        true
    }

    fn matches_archetype_and(archetype: &Archetype) -> bool {
        for typle_index!(i) in 0..T::LEN {
            if !T::<{ i }>::matches_archetype(archetype) {
                return false;
            }
        }
        true
    }

    fn matches_archetype_or(archetype: &Archetype) -> bool {
        for typle_index!(i) in 0..T::LEN {
            if T::<{ i }>::matches_archetype(archetype) {
                return true;
            }
        }
        false
    }

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
    // A component can only have been added on an entity that actually has it,
    // so archetypes lacking the component can be pruned. The per-tick decision
    // is still row-level, hence not `is_archetypal`.
    fn matches_archetype(archetype: &Archetype) -> bool {
        archetype.contains_all(&T::get_component_ids())
    }

    fn filter<'w>(world: UnsafeWorldCell<'w>, entity: Entity) -> bool {
        for component_id in T::get_component_ids() {
            if !world.world().was_component_added(entity, component_id) {
                return false;
            }
        }
        true
    }

    fn filter_or<'w>(world: UnsafeWorldCell<'w>, entity: Entity) -> bool {
        Self::filter(world, entity)
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
    fn matches_archetype(archetype: &Archetype) -> bool {
        archetype.contains_all(&T::get_component_ids())
    }

    fn filter<'w>(world: UnsafeWorldCell<'w>, entity: Entity) -> bool {
        for component_id in T::get_component_ids() {
            if !world.world().was_component_changed(entity, component_id) {
                return false;
            }
        }
        true
    }

    fn filter_or<'w>(world: UnsafeWorldCell<'w>, entity: Entity) -> bool {
        Self::filter(world, entity)
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
    // Component presence is identical for every entity in an archetype, so this
    // is decided once per archetype and never per entity.
    fn is_archetypal() -> bool {
        true
    }

    fn matches_archetype(archetype: &Archetype) -> bool {
        archetype.contains_all(&T::get_component_ids())
    }

    fn filter<'w>(world: UnsafeWorldCell<'w>, entity: Entity) -> bool {
        let world = world.world();

        match world.entity_store().find_location(entity) {
            Some(location) => {
                let archetypes = world.archetypes();
                let archetype = &archetypes[location.archetype_index as usize];
                archetype.contains_all(&T::get_component_ids())
            }
            None => false,
        }
    }

    fn filter_or<'w>(world: UnsafeWorldCell<'w>, entity: Entity) -> bool {
        Self::filter(world, entity)
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
    // `Not` can only be resolved at the archetype level when the inner filter is
    // itself archetypal; otherwise we cannot prune and must fall back to the
    // per-entity check.
    fn is_archetypal() -> bool {
        T::is_archetypal()
    }

    fn matches_archetype(archetype: &Archetype) -> bool {
        if T::is_archetypal() {
            !T::matches_archetype(archetype)
        } else {
            true
        }
    }

    fn filter<'w>(world: UnsafeWorldCell<'w>, entity: Entity) -> bool {
        !T::filter(world, entity)
    }

    fn filter_or<'w>(world: UnsafeWorldCell<'w>, entity: Entity) -> bool {
        Self::filter(world, entity)
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
    // `Or` is fully archetypal only when every branch is; otherwise a row-level
    // branch forces per-entity evaluation.
    fn is_archetypal() -> bool {
        T::is_archetypal()
    }

    fn matches_archetype(archetype: &Archetype) -> bool {
        T::matches_archetype_or(archetype)
    }

    fn filter<'w>(world: UnsafeWorldCell<'w>, entity: Entity) -> bool {
        T::filter_or(world, entity)
    }

    fn filter_or<'w>(world: UnsafeWorldCell<'w>, entity: Entity) -> bool {
        Self::filter(world, entity)
    }
}
