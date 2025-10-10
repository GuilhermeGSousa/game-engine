use std::ops::Deref;

use ecs::{
    component::{Component, ComponentLifecycleCallback},
    entity::Entity,
};

pub struct RenderEntity(Entity);

impl RenderEntity {
    pub fn new(entity: Entity) -> Self
    {
        Self(entity)
    }
}

impl Deref for RenderEntity {
    type Target = Entity;

    fn deref(&self) -> &Self::Target {
        &self.0
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
                world.despawn(**render_entity);
            }
        })
    }

    fn on_add() -> Option<ComponentLifecycleCallback> {
        None
    }
}
