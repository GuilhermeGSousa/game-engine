use std::{cell::UnsafeCell, collections::HashMap, marker::PhantomData, ptr};

use anymap::AnyMap;

use crate::{
    archetype::Archetype,
    bundle::ComponentBundle,
    common::generate_type_id,
    component::{Component, ComponentId},
    entity::{Entity, EntityLocation, EntityType},
    entity_store::EntityStore,
    resource::Resource,
    system::system_input::SystemInput,
};

pub struct World {
    archetypes: Vec<Archetype>,
    resources: AnyMap,

    entity_store: EntityStore,
    archetype_index: HashMap<EntityType, usize>,
    current_tick: u32,
}

impl World {
    pub fn new() -> World {
        Self {
            archetypes: Vec::new(),
            archetype_index: HashMap::new(),
            resources: AnyMap::new(),
            entity_store: EntityStore::new(),
            current_tick: 0,
        }
    }

    pub fn spawn<T: ComponentBundle>(&mut self, bundle: T) -> Entity {
        let type_ids = T::get_component_ids();
        let entity_type = generate_type_id(&type_ids);

        let entity = self.entity_store.alloc();

        let archetype_index = self
            .archetype_index
            .entry(entity_type.clone())
            .or_insert_with(|| {
                let archetype = Archetype::new(T::generate_empty_table());
                self.archetypes.push(archetype);
                self.archetypes.len() - 1
            });

        let archetype: &mut Archetype = &mut self.archetypes[*archetype_index];

        let table_row = bundle.add_row_to_archetype(archetype, entity, self.current_tick);

        let new_location = EntityLocation {
            archetype_index: *archetype_index as u32,
            row: table_row,
        };

        self.entity_store.set_location(entity, new_location);

        entity
    }

    pub fn despawn(&mut self, entity: Entity) {
        match self.entity_store.find_location(entity) {
            Some(location) => {
                let archetype = &mut self.archetypes[location.archetype_index as usize];
                let swapped_entity = archetype.remove_swap(location.row);
                self.entity_store.set_location(swapped_entity, location);
                self.entity_store.free(entity);
            }
            None => panic!("Entity should exist in the world"),
        }
    }

    pub fn get_archetypes(&self) -> &Vec<Archetype> {
        &self.archetypes
    }

    pub fn get_archetypes_mut(&mut self) -> &mut Vec<Archetype> {
        &mut self.archetypes
    }

    pub(crate) fn get_entity_store(&self) -> &EntityStore {
        &self.entity_store
    }

    pub fn get_component_for_entity<T: Component>(&self, entity: Entity) -> Option<&T> {
        self.entity_store
            .find_location(entity)
            .map(|location| self.get_component_for_entity_location(location))
            .flatten()
    }

    pub fn get_component_for_entity_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        self.entity_store
            .find_location(entity)
            .map(|location| self.get_component_for_entity_location_mut(location))
            .flatten()
    }

    pub(crate) fn get_component_for_entity_location<T: Component>(
        &self,
        entity_location: EntityLocation,
    ) -> Option<&T> {
        self.archetypes
            .get(entity_location.archetype_index as usize)
            .map(|archetype| unsafe { archetype.get_component_unsafe(entity_location.row) })
            .flatten()
    }

    pub(crate) fn get_component_for_entity_location_mut<T: Component>(
        &mut self,
        entity_location: EntityLocation,
    ) -> Option<&mut T> {
        self.archetypes
            .get_mut(entity_location.archetype_index as usize)
            .map(|archetype| unsafe {
                archetype.get_component_unsafe_mut(entity_location.row, self.current_tick)
            })
            .flatten()
    }

    pub fn insert_resource<T: Resource>(&mut self, resource: T) {
        self.resources.insert(resource);
    }

    pub fn remove_resource<T: Resource + 'static>(&mut self) -> Option<T> {
        self.resources.remove()
    }

    pub fn get_resource<T: 'static>(&self) -> Option<&T> {
        self.resources.get()
    }

    pub fn get_resource_mut<T: 'static>(&mut self) -> Option<&mut T> {
        self.resources.get_mut()
    }

    pub fn as_unsafe_world_cell_mut(&mut self) -> UnsafeWorldCell<'_> {
        UnsafeWorldCell::new_mutable(self)
    }

    pub fn as_unsafe_world_cell_ref(&self) -> UnsafeWorldCell<'_> {
        UnsafeWorldCell::new_ref(self)
    }

    pub fn tick(&mut self) {
        self.current_tick += 1;
    }

    pub fn was_component_added(&self, entity: Entity, component_id: ComponentId) -> bool {
        if let Some(location) = self.entity_store.find_location(entity) {
            self.archetypes[location.archetype_index as usize].was_entity_added(
                component_id,
                location.row,
                self.current_tick,
            )
        } else {
            false
        }
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

unsafe impl SystemInput for &World {
    type State = ();
    type Data<'world> = &'world World;

    fn init_state() -> Self::State {
        ()
    }

    unsafe fn get_data<'world>(world: UnsafeWorldCell<'world>) -> Self::Data<'world> {
        world.get_world()
    }
}

unsafe impl SystemInput for &mut World {
    type State = ();
    type Data<'world> = &'world mut World;

    fn init_state() -> Self::State {
        ()
    }

    unsafe fn get_data<'world>(world: UnsafeWorldCell<'world>) -> Self::Data<'world> {
        world.get_world_mut()
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
