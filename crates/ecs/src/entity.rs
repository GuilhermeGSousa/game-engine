#[derive(Eq, Hash, PartialEq, Clone, Copy)]
pub struct Entity(pub usize);

#[derive(Eq, Hash, PartialEq, Clone)]
pub struct EntityType(pub u64);

pub struct EntityRecord {
    pub archetype_index: usize,
    pub row_index: usize,
}

impl EntityRecord {
    pub fn new(archetype_index: usize, row_index: usize) -> EntityRecord {
        EntityRecord {
            archetype_index: archetype_index,
            row_index: row_index,
        }
    }
}
