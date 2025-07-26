use std::ops::Deref;

use ecs::{
    component::{Component, ComponentLifecycleCallback},
    entity::Entity,
};

pub enum RenderEntity {
    Uninitialized,
    Initialized(Entity),
}

impl RenderEntity {
    pub fn new() -> Self {
        RenderEntity::Uninitialized
    }

    pub fn set_entity(&mut self, entity: Entity) {
        match self {
            RenderEntity::Uninitialized => *self = RenderEntity::Initialized(entity),
            RenderEntity::Initialized(e) => *e = entity,
        }
    }
}

impl Component for RenderEntity {
    fn name() -> String {
        String::from("RenderEntity")
    }

    fn on_remove() -> Option<ComponentLifecycleCallback> {
        Some(|mut world, context| {
            if let Some(render_entity) =
                world.get_component_for_entity::<RenderEntity>(context.entity)
            {
                if let RenderEntity::Initialized(other_entity) = render_entity {
                    world.despawn(*other_entity);
                }
            }
        })
    }

    fn on_add() -> Option<ComponentLifecycleCallback> {
        None
    }
}
