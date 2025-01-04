#[derive(Eq, Hash, PartialEq, Clone)]
pub struct Entity(pub usize);

#[derive(Eq, Hash, PartialEq, Clone)]
pub struct EntityType(pub u64);

pub struct EntityRecord {
    pub archetype_index: usize,
    pub column: usize,
}

impl EntityRecord {
    pub fn new(archetype_index: usize, column: usize) -> EntityRecord {
        EntityRecord {
            archetype_index: archetype_index,
            column: column,
        }
    }
}
