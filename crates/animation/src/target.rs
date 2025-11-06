use ecs::{component::Component, entity::Entity};
use uuid::Uuid;

#[derive(Component)]
pub struct AnimationTarget {
    id: Uuid,
    animator: Entity,
}
