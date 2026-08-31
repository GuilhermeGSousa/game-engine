use anyhow::Ok;
use facet::{AllocError, Facet, HeapValue, Partial, Shape};

use crate::{Component, Entity, World};

#[derive(Clone)]
pub(crate) struct ComponentReflection {
    shape: &'static Shape,
    insert_fn: fn(HeapValue, &mut World, Entity) -> anyhow::Result<()>,
}

fn insertion_typed<T: Component + for<'a> Facet<'a>>(
    heap: HeapValue,
    world: &mut World,
    entity: Entity,
) -> anyhow::Result<()> {
    world.insert(heap.materialize::<T>()?, entity);
    Ok(())
}

impl ComponentReflection {
    pub(crate) fn from_type<T: Component + for<'a> Facet<'a>>() -> Self {
        Self {
            shape: T::SHAPE,
            insert_fn: insertion_typed::<T>,
        }
    }

    pub(crate) fn alloc_shape(&self) -> Result<Partial<'_>, AllocError> {
        // This is safe, if the shape exists it is valid
        unsafe { Partial::alloc_shape(self.shape) }
    }

    pub(crate) fn insert(
        &self,
        value: HeapValue,
        world: &mut World,
        entity: Entity,
    ) -> anyhow::Result<()> {
        (self.insert_fn)(value, world, entity)
    }
}
