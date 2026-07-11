use ecs::component::Component;

use crate::body::BodyId;

#[allow(dead_code)]
#[derive(Component)]
pub struct Collider(pub(crate) BodyId);

impl Collider {}
