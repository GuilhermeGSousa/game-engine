use std::marker::PhantomData;

use crate::{component::bundle::ComponentBundle, entity::Entity, world::UnsafeWorldCell};
use typle::typle;

pub trait QueryFilter {
    fn filter<'w>(_world: UnsafeWorldCell<'w>, _entity: Entity) -> bool {
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
    fn filter<'w>(world: UnsafeWorldCell<'w>, entity: Entity) -> bool {
        for typle_index!(i) in 0..T::LEN {
            if !T::<{ i }>::filter(world, entity) {
                return false;
            }
        }
        true
    }
}

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
