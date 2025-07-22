use ecs::{component::Component, entity::Entity};

#[derive(Component)]
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
