use std::ops::{Deref, DerefMut};

use ecs::component::Component;
use essential::transform::Transform;
use rapier3d::{
    math::Vector,
    prelude::{RigidBodyBuilder, RigidBodyHandle},
};

use crate::physics_state::PhysicsState;

#[derive(Component)]
pub struct RigidBody(RigidBodyHandle);

impl RigidBody {
    pub fn new(transform: &Transform, state: &mut PhysicsState) -> Self {
        let pos = transform.translation;
        let rb = RigidBodyBuilder::dynamic()
            .translation(Vector::new(pos.x, pos.y, pos.z))
            .enabled(true)
            .build();
        Self(state.rigid_body_set.insert(rb))
    }
}

impl Deref for RigidBody {
    type Target = RigidBodyHandle;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RigidBody {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
