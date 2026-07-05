use ecs::component::Component;

use crate::body::BodyId;

/// A collider attached to a body. For dynamic colliders the [`BodyId`] is the
/// parent rigid body's id; for static colliders it is the standalone static
/// body created to hold the shape.
#[allow(dead_code)]
#[derive(Component)]
pub struct Collider(pub(crate) BodyId);

impl Collider {}
