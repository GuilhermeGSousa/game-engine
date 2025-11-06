use ecs::{component::Component, entity::Entity};
use uuid::Uuid;

#[derive(Component)]
pub struct AnimationTarget {
    pub id: Uuid,
    pub animator: Entity,
}
