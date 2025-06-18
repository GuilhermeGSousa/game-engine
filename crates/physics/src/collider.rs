use ecs::component::Component;
use rapier3d::prelude::ColliderHandle;

#[derive(Component)]
pub struct Collider(pub(crate) ColliderHandle);

impl Collider {}
