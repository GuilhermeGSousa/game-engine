use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::{component::Component, entity::Entity, world::RestrictedWorld};

/// A reference to another node of the same `Scene`, by node index. Resolved
/// to a real [`Entity`] during spawning via [`SceneSpawnContext::entity_for`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SceneEntityRef(pub usize);

/// What a [`SceneComponent`] gets while a scene is being spawned.
pub struct SceneSpawnContext<'w> {
    world: RestrictedWorld<'w>,
    node_entities: &'w [Entity],
}

impl<'w> SceneSpawnContext<'w> {
    pub fn new(world: RestrictedWorld<'w>, node_entities: &'w [Entity]) -> Self {
        Self {
            world,
            node_entities,
        }
    }

    /// Adds a runtime component to any entity in the scene being spawned —
    /// not necessarily the one currently being applied to.
    pub fn insert<T: Component>(&mut self, component: T, entity: Entity) {
        self.world.insert(component, entity, true);
    }

    /// Resolves a node reference to its spawned entity. Returns `None` for an
    /// out-of-range index, so a malformed scene cannot panic.
    pub fn entity_for(&self, reference: SceneEntityRef) -> Option<Entity> {
        self.node_entities.get(reference.0).copied()
    }

    /// Escape hatch for resources — notably `AssetServer`, which `ecs` cannot
    /// name.
    pub fn world(&mut self) -> &mut RestrictedWorld<'w> {
        &mut self.world
    }
}

/// Data authored into a `Scene` that knows how to apply itself to a
/// spawned entity.
///
/// A type that is a runtime component inserts itself. A type that is really
/// authoring data expands into several runtime components — possibly on other
/// entities — and never inserts one of itself. Both are this one interface.
pub trait SceneComponent: Component + DeserializeOwned + Sized + 'static {
    fn apply(self, entity: Entity, ctx: &mut SceneSpawnContext<'_>);
}
