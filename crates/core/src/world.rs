use std::{cell::UnsafeCell, collections::HashMap, marker::PhantomData, ptr};

use crate::{
    archetype::Archetype,
    bundle::ComponentBundle,
    common::generate_type_id,
    entity::{Entity, EntityRecord, EntityType},
    resource::{Resource, ResourceId, ToAny},
};

pub struct World {
    entity_count: usize,
    archetypes: Vec<Archetype>,
    resources: HashMap<ResourceId, Box<dyn Resource>>,

    // We need
    // map entity to archetype
    // map set of components to archetype
    entity_index: HashMap<Entity, EntityRecord>,
    archetype_index: HashMap<EntityType, usize>,
}

impl World {
    pub fn new() -> World {
        Self {
            entity_count: 0,
            archetypes: Vec::new(),
            entity_index: HashMap::new(),
            archetype_index: HashMap::new(),
            resources: HashMap::new(),
        }
    }

    pub fn spawn<T: ComponentBundle>(&mut self, bundle: T) -> Entity {
        let entity = Entity(self.entity_count);
        self.entity_count += 1;

        let type_ids = T::get_type_ids();
        let entity_type = generate_type_id(&type_ids);

        let archetype_index = self
            .archetype_index
            .entry(entity_type.clone())
            .or_insert_with(|| {
                let archetype = Archetype::new(T::generate_empty_table());
                self.archetypes.push(archetype);
                self.archetypes.len() - 1
            });

        let _archetype: &mut Archetype = &mut self.archetypes[*archetype_index];
        bundle.get_components(|type_id, raw_value| {
            _archetype.add_component(type_id, raw_value);
        });

        let entity_record = EntityRecord::new(*archetype_index, _archetype.get_row_count() - 1);
        self.entity_index.insert(entity.clone(), entity_record);
        entity
    }

    pub fn get_archetypes(&self) -> &Vec<Archetype> {
        &self.archetypes
    }

    pub fn get_archetypes_mut(&mut self) -> &mut Vec<Archetype> {
        &mut self.archetypes
    }

    pub fn insert_resource<T: Resource>(&mut self, resource: T) {
        self.resources
            .insert(ResourceId::of::<T>(), Box::new(resource));
    }

    pub fn get_resource<T: 'static>(&self) -> Option<&T> {
        self.resources
            .get(&ResourceId::of::<T>())
            .and_then(|resource| resource.as_any().downcast_ref())
    }

    pub fn get_resource_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.resources
            .get_mut(&ResourceId::of::<T>())
            .and_then(|resource| resource.as_any_mut().downcast_mut())
    }

    pub fn as_unsafe_world_cell_mut(&mut self) -> UnsafeWorldCell<'_> {
        UnsafeWorldCell::new_mutable(self)
    }

    pub fn as_unsafe_world_cell_ref(&self) -> UnsafeWorldCell<'_> {
        UnsafeWorldCell::new_ref(self)
    }
}

#[derive(Copy, Clone)]
pub struct UnsafeWorldCell<'w> {
    ptr: *mut World,
    is_mutable: bool,
    _marker: PhantomData<(&'w World, &'w UnsafeCell<World>)>,
}

impl<'w> From<&'w mut World> for UnsafeWorldCell<'w> {
    fn from(value: &'w mut World) -> Self {
        value.as_unsafe_world_cell_mut()
    }
}

impl<'w> From<&'w World> for UnsafeWorldCell<'w> {
    fn from(value: &'w World) -> Self {
        value.as_unsafe_world_cell_ref()
    }
}

impl<'w> UnsafeWorldCell<'w> {
    fn assert_mutable(&self) {
        debug_assert!(self.is_mutable, "UnsafeWorldCell is not mutable");
    }

    pub(crate) fn new_mutable(world: &'w mut World) -> Self {
        Self {
            ptr: ptr::from_mut(world),
            is_mutable: true,
            _marker: PhantomData,
        }
    }

    pub(crate) fn new_ref(world: &'w World) -> Self {
        Self {
            ptr: ptr::from_ref(world).cast_mut(),
            is_mutable: false,
            _marker: PhantomData,
        }
    }

    pub fn get_world(&self) -> &'w World {
        unsafe { &*self.ptr }
    }

    pub fn get_world_mut(&self) -> &'w mut World {
        self.assert_mutable();
        unsafe { &mut *self.ptr }
    }
}
