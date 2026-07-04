use std::ops::{Deref, DerefMut};

use ecs::component::Component;
use essential::transform::Transform;
use glam::Vec3;
use rapier3d::{
    math::Vector,
    prelude::{RigidBodyBuilder, RigidBodyHandle},
};

use crate::physics_state::PhysicsState;

#[derive(Component)]
pub struct RigidBody(RigidBodyHandle);

impl RigidBody {
    pub fn new_dynamic(transform: &Transform, state: &mut PhysicsState) -> Self {
        let pos = transform.translation;
        let rb = RigidBodyBuilder::dynamic()
            .translation(Vector::new(pos.x, pos.y, pos.z))
            .enabled(true)
            .build();
        Self(state.rigid_body_set.insert(rb))
    }

    pub fn new_static(transform: &Transform, state: &mut PhysicsState) -> Self {
        let pos = transform.translation;
        let rb = RigidBodyBuilder::fixed()
            .translation(Vector::new(pos.x, pos.y, pos.z))
            .enabled(true)
            .build();
        Self(state.rigid_body_set.insert(rb))
    }

    pub fn new_kinematic_position_based(transform: &Transform, state: &mut PhysicsState) -> Self {
        let pos = transform.translation;
        let rb = RigidBodyBuilder::kinematic_position_based()
            .translation(Vector::new(pos.x, pos.y, pos.z))
            .enabled(true)
            .build();
        Self(state.rigid_body_set.insert(rb))
    }

    /// Drives a kinematic body's position for the next physics step. No-op for non-kinematic
    /// bodies. Used to move platforms/elevators without going through a [`CharacterController`](crate::character_controller::CharacterController).
    pub fn set_next_kinematic_translation(&self, translation: Vec3, state: &mut PhysicsState) {
        if let Some(body) = state.rigid_body_set.get_mut(self.0) {
            body.set_next_kinematic_translation(Vector::new(
                translation.x,
                translation.y,
                translation.z,
            ));
        }
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
