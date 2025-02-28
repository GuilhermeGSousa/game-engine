use std::collections::HashMap;

use crate::{
    archetype::Archetype,
    bundle::ComponentBundle,
    common::generate_type_id,
    entity::{Entity, EntityRecord, EntityType},
};

pub struct World {
    entity_count: usize,
    archetypes: Vec<Archetype>,

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
}
