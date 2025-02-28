use std::collections::HashMap;

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
}
