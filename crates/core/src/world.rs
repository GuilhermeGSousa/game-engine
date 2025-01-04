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

    pub fn spawn<T: ComponentBundle>(&mut self, _bundle: T) -> Entity {
        let entity = Entity(self.entity_count);
        self.entity_count += 1;

        let type_ids = T::get_type_ids();
        let entity_type = generate_type_id(&type_ids);

        let archetype_index = self
            .archetype_index
            .entry(entity_type.clone())
            .or_insert_with(|| {
                let archetype = Archetype::new(entity_type);
                self.archetypes.push(archetype);
                self.archetypes.len() - 1
            });

        let _archetype = &mut self.archetypes[*archetype_index];
        // Add components to archetype

        let entity_record = EntityRecord::new(*archetype_index, 0);
        self.entity_index.insert(entity.clone(), entity_record);
        entity
    }
}
