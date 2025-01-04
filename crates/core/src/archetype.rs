use crate::component::ComponentStorage;
use crate::entity::EntityType;

pub struct Archetype {
    pub entity_type: EntityType,
    pub components: Vec<Box<dyn ComponentStorage>>,
}

impl Archetype {
    pub fn new(entity_type: EntityType) -> Archetype {
        Archetype {
            entity_type: entity_type,
            components: Vec::new(),
        }
    }

    pub fn remove_column(&mut self, column: usize) {
        for component in self.components.iter_mut() {
            component.remove(column);
        }
    }
}
