use ecs::component::Component;
use rapier3d::prelude::ColliderHandle;

#[allow(dead_code)]
#[derive(Component)]
pub struct Collider(pub(crate) ColliderHandle);

impl Collider {}
