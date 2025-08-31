use any_vec::any_value::AnyValueWrapper;
use anymap3::AnyMap;
use std::collections::hash_map::Entry::{Occupied, Vacant};
use std::{
    any::TypeId, cell::UnsafeCell, collections::HashMap, marker::PhantomData, ops::Deref, ptr,
};

use crate::component::bundle::ComponentBundle;
use crate::component::Tick;
use crate::entity::entity_store::EntityStore;
use crate::entity::hierarchy::{ChildOf, Children};
use crate::resource::ResourceStorage;
use crate::table::MutableCellAccessor;
use crate::{
    archetype::Archetype,
    common::generate_type_id,
    component::{Component, ComponentId, ComponentLifecycleCallbacks, ComponentLifecycleContext},
    entity::{Entity, EntityLocation, EntityType},
    resource::Resource,
    system::system_input::SystemInput,
    table::{Table, TableRowIndex},
    utilities::TypeIdMap,
};

pub struct World {
    archetypes: Vec<Archetype>,
    resources: AnyMap,
    entity_store: EntityStore,
    archetype_index: HashMap<EntityType, usize>,
    component_lifetimes: TypeIdMap<ComponentLifecycleCallbacks>,
    current_tick: u32,
}

impl World {
    pub fn new() -> World {
        Self {
            archetypes: Vec::new(),
            archetype_index: HashMap::new(),
            resources: AnyMap::new(),
            component_lifetimes: Default::default(),
            entity_store: EntityStore::new(),
            current_tick: 0,
        }
    }

    pub fn spawn<T: ComponentBundle>(&mut self, bundle: T) -> Entity {
        let entity = self.entity_store.alloc();

        self.spawn_allocated(entity, bundle);

        entity
    }

    pub(crate) fn spawn_allocated<T: ComponentBundle>(&mut self, entity: Entity, bundle: T) {
        let type_ids = T::get_component_ids();
        let entity_type = generate_type_id(&type_ids);

        let archetype_index = self
            .archetype_index
            .entry(entity_type.clone())
            .or_insert_with(|| {
                let archetype = Archetype::new(T::generate_empty_table(), type_ids);
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
    }

    pub fn despawn(&mut self, entity: Entity) {
        match self.entity_store.find_location(entity) {
            Some(location) => {
                {
                    let cell = self.as_unsafe_world_cell_mut();
                    cell.trigger_on_remove(entity, location);
                }

                let archetype = &mut self.archetypes[location.archetype_index as usize];

                if let Some(swapped_entity) = archetype.entities().last() {
                    self.entity_store.set_location(*swapped_entity, location);
                }

                archetype.remove_swap(location.row);

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

    pub fn insert_component<T: Component>(&mut self, component: T, entity: Entity) {
        match self.entity_store.find_location(entity) {
            Some(location) => {
                let previous_archetype = &mut self.archetypes[location.archetype_index as usize];

                let inserted_id = TypeId::of::<T>();
                let mut component_ids = previous_archetype.component_ids().to_vec();
                component_ids.push(inserted_id);

                let entity_type = generate_type_id(&component_ids);

                // Remove row from previous archetype
                let mut removed_row = previous_archetype.remove_swap(location.row);

                // Add new component to the removed row
                removed_row.insert(inserted_id, AnyValueWrapper::<T>::new(component));

                // Add row to new archetype
                let archetype_index = match self.archetype_index.entry(entity_type.clone()) {
                    Occupied(occupied_entry) => {
                        let new_archetype_index = *occupied_entry.get();
                        let new_archetype = &mut self.archetypes[*occupied_entry.get() as usize];
                        new_archetype.add_row(removed_row, self.current_tick);
                        new_archetype_index
                    }
                    Vacant(vacant_entry) => {
                        let new_archetype_index = self.archetypes.len();
                        let archetype = Archetype::new(
                            Table::from_row(removed_row, self.current_tick),
                            component_ids,
                        );
                        self.archetypes.push(archetype);
                        vacant_entry.insert(new_archetype_index);
                        new_archetype_index
                    }
                };

                // Store in entity store
                self.entity_store.set_location(
                    entity,
                    EntityLocation {
                        archetype_index: archetype_index as u32,
                        row: TableRowIndex::new(self.archetypes[archetype_index].len() - 1),
                    },
                );
            }
            None => panic!("Entity should exist in the world"),
        }
    }

    pub(crate) fn get_entity_store(&self) -> &EntityStore {
        &self.entity_store
    }

    pub(crate) fn get_entity_store_mut(&mut self) -> &mut EntityStore {
        &mut self.entity_store
    }

    pub fn get_component_for_entity<T: Component>(&self, entity: Entity) -> Option<&T> {
        self.entity_store
            .find_location(entity)
            .map(|location| self.get_component_for_entity_location(location))
            .flatten()
    }

    pub fn get_component_for_entity_mut<T: Component>(&mut self, entity: Entity) -> Option<&mut T> {
        let current_tick = self.current_tick;
        self.entity_store
            .find_location(entity)
            .map(|location| {
                self.get_component_for_entity_location_mut(location)
                    .map(|accessor| {
                        accessor.changed_tick.set(current_tick);
                        accessor.data
                    })
            })
            .flatten()
    }

    pub(crate) fn get_component_accessor_for_entity_mut<T: Component>(
        &mut self,
        entity: Entity,
    ) -> Option<MutableCellAccessor<T>> {
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
    ) -> Option<MutableCellAccessor<T>> {
        self.archetypes
            .get_mut(entity_location.archetype_index as usize)
            .map(|archetype| unsafe { archetype.get_component_unsafe_mut(entity_location.row) })
            .flatten()
    }

    pub fn insert_resource<T: Resource>(&mut self, resource: T) {
        self.resources
            .insert(ResourceStorage::new(resource, self.current_tick));
    }

    pub fn remove_resource<T: Resource + 'static>(&mut self) -> Option<T> {
        self.resources
            .remove::<ResourceStorage<T>>()
            .map(|resource_storage| resource_storage.data)
    }

    pub fn get_resource<T: Resource + 'static>(&self) -> Option<&T> {
        self.resources
            .get::<ResourceStorage<T>>()
            .map(|resource_storage| &resource_storage.data)
    }

    pub fn get_resource_mut<T: Resource + 'static>(&mut self) -> Option<&mut T> {
        self.resources
            .get_mut::<ResourceStorage<T>>()
            .map(|resource_storage| &mut resource_storage.data)
    }

    pub(crate) fn get_resource_storage<T: Resource + 'static>(
        &self,
    ) -> Option<&ResourceStorage<T>> {
        self.resources.get()
    }

    pub(crate) fn get_resource_storage_mut<T: Resource + 'static>(
        &mut self,
    ) -> Option<&mut ResourceStorage<T>> {
        self.resources.get_mut()
    }

    pub fn as_unsafe_world_cell_mut(&mut self) -> UnsafeWorldCell<'_> {
        UnsafeWorldCell::new_mutable(self)
    }

    pub fn as_unsafe_world_cell(&self) -> UnsafeWorldCell<'_> {
        UnsafeWorldCell::new_ref(self)
    }

    pub fn tick(&mut self) {
        self.current_tick += 1;
    }

    pub fn current_tick(&self) -> Tick {
        Tick::new(self.current_tick)
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

    pub fn was_component_changed(&self, entity: Entity, component_id: ComponentId) -> bool {
        if let Some(location) = self.entity_store.find_location(entity) {
            self.archetypes[location.archetype_index as usize].was_entity_changed(
                component_id,
                location.row,
                self.current_tick,
            )
        } else {
            false
        }
    }

    pub fn register_component_lifetimes<T: Component>(&mut self) {
        self.component_lifetimes
            .entry(ComponentId::of::<T>())
            .or_insert(ComponentLifecycleCallbacks::from_component::<T>());
    }

    pub fn add_child(&mut self, parent: Entity, child: Entity) {
        self.insert_component(ChildOf::new(parent), child);

        match self.get_component_accessor_for_entity_mut::<Children>(parent) {
            Some(table_cell) => {
                table_cell.data.add_child(child);
            }
            None => {
                self.insert_component(Children::from_children(vec![child]), parent);
            }
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
        value.as_unsafe_world_cell()
    }
}

unsafe impl SystemInput for &World {
    type State = ();
    type Data<'world, 'state> = &'world World;

    fn init_state() -> Self::State {
        ()
    }

    unsafe fn get_data<'world, 'state>(
        _state: &'state mut Self::State,
        world: UnsafeWorldCell<'world>,
    ) -> Self::Data<'world, 'state> {
        world.world()
    }
}

unsafe impl SystemInput for &mut World {
    type State = ();
    type Data<'world, 'state> = &'world mut World;

    fn init_state() -> Self::State {
        ()
    }

    unsafe fn get_data<'world, 'state>(
        _state: &'state mut Self::State,
        world: UnsafeWorldCell<'world>,
    ) -> Self::Data<'world, 'state> {
        world.world_mut()
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

    pub fn world(&self) -> &'w World {
        unsafe { &*self.ptr }
    }

    pub fn world_mut(&self) -> &'w mut World {
        self.assert_mutable();
        unsafe { &mut *self.ptr }
    }

    pub fn into_restricted(self) -> RestrictedWorld<'w> {
        RestrictedWorld { world_cell: self }
    }

    pub fn trigger_on_remove(&self, entity: Entity, location: EntityLocation) {
        let world = self.world();
        let archetype = &world.archetypes[location.archetype_index as usize];

        for id in archetype.component_ids() {
            if let Some(lifetimes) = world.component_lifetimes.get(id) {
                if let Some(remove) = lifetimes.on_remove {
                    remove(self.into_restricted(), ComponentLifecycleContext { entity });
                }
            }
        }
    }
}

pub struct RestrictedWorld<'w> {
    world_cell: UnsafeWorldCell<'w>,
}

impl<'w> RestrictedWorld<'w> {
    pub fn despawn(&mut self, entity: Entity) {
        // TODO: Use commands instead
        self.world_cell.world_mut().despawn(entity);
    }
}

impl<'w> From<&'w mut World> for RestrictedWorld<'w> {
    fn from(world: &'w mut World) -> RestrictedWorld<'w> {
        RestrictedWorld {
            world_cell: world.as_unsafe_world_cell(),
        }
    }
}

impl<'w> Deref for RestrictedWorld<'w> {
    type Target = World;

    fn deref(&self) -> &Self::Target {
        self.world_cell.world()
    }
}
