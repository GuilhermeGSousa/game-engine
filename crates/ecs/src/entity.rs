use std::num::NonZero;

use crate::table::TableRowIndex;

#[derive(Eq, Hash, PartialEq, Clone, Copy)]
pub struct Entity {
    index: u32,
    generation: NonZero<u32>,
}

impl Entity {
    pub(crate) fn new(index: u32, generation: NonZero<u32>) -> Self {
        Self { index, generation }
    }

    pub fn index(&self) -> u32 {
        self.index
    }

    pub fn generation(&self) -> NonZero<u32> {
        self.generation
    }
}

#[derive(Eq, Hash, PartialEq, Clone)]
pub struct EntityType(pub u64);

#[derive(Clone, Copy, PartialEq)]
pub struct EntityLocation {
    pub(crate) archetype_index: u32,
    pub(crate) row: TableRowIndex,
}

impl EntityLocation {
    pub(crate) const INVALID: EntityLocation = EntityLocation {
        archetype_index: u32::MAX,
        row: TableRowIndex::new(u32::MAX),
    };
}
