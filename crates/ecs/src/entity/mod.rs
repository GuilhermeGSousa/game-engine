use std::{
    fmt::{Debug, Display},
    num::NonZero,
};

use crate::table::TableRowIndex;

pub mod entity_store;
pub mod hierarchy;

/// A lightweight, copyable handle that uniquely identifies a game object in the [`World`](crate::world::World).
///
/// Entities are created with [`World::spawn`](crate::world::World::spawn) and destroyed with
/// [`World::despawn`](crate::world::World::despawn).  An entity is just an `(index, generation)`
/// pair — the generation is bumped each time a slot is reused so stale handles can be detected.
#[derive(Eq, Hash, PartialEq, Clone, Copy)]
pub struct Entity {
    index: u32,
    generation: NonZero<u32>,
}

impl Entity {
    pub(crate) fn new(index: u32, generation: NonZero<u32>) -> Self {
        Self { index, generation }
    }

    /// Returns the slot index of this entity within the entity store.
    pub fn index(&self) -> u32 {
        self.index
    }

    /// Returns the generation counter that distinguishes this entity from previously-occupying
    /// entities at the same index.
    pub fn generation(&self) -> NonZero<u32> {
        self.generation
    }
}

impl Debug for Entity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Entity")
            .field("index", &self.index)
            .field("generation", &self.generation)
            .finish()
    }
}

impl Display for Entity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Entity(index: {}, gen: {})", self.index, self.generation)
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
        row: TableRowIndex::new(usize::MAX),
    };
}
