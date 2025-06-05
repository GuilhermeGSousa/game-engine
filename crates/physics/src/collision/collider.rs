use ecs::component::Component;

use crate::collision::collider_shape::CollisionShape;

#[derive(Component)]
pub struct Collider {
    pub shape: CollisionShape,
}
