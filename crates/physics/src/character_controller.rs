use ecs::{component::Component, query::Query, resource::ResMut};
use essential::{time::Time, transform::Transform};
use glam::Vec3;
use rapier3d::{
    control::KinematicCharacterController,
    math::{Isometry, Rotation, Translation},
    pipeline::QueryFilter,
    prelude::nalgebra::Quaternion,
};

use crate::{collider::Collider, physics_state::PhysicsState, rigid_body::RigidBody};

/// Drives a kinematic capsule character around the world: gravity, jumping, and
/// collide-and-slide movement against the rest of the physics world.
///
/// Attach alongside a kinematic-position-based [`RigidBody`], a capsule [`Collider`]
/// (see [`PhysicsState::make_capsule`]) and a [`Transform`].
#[derive(Component)]
pub struct CharacterController {
    /// Horizontal move intent for the current frame, in world-space units per second.
    /// Written by gameplay code every frame; this system consumes it and does not reset it.
    pub desired_translation: Vec3,
    /// Initial upward speed (units/second) applied when a jump is consumed.
    pub jump_speed: f32,
    /// Downward acceleration (units/second^2) applied while airborne.
    pub gravity: f32,
    /// Set by gameplay to request a jump on the next update; cleared once consumed.
    pub jump_requested: bool,
    /// Whether the character was touching the ground after the last update.
    pub grounded: bool,
    vertical_velocity: f32,
    controller: KinematicCharacterController,
}

impl CharacterController {
    pub fn new() -> Self {
        Self {
            desired_translation: Vec3::ZERO,
            jump_speed: 6.0,
            gravity: 20.0,
            jump_requested: false,
            grounded: false,
            vertical_velocity: 0.0,
            controller: KinematicCharacterController::default(),
        }
    }

    /// Requests a jump; only takes effect if the character is grounded on the next update.
    pub fn request_jump(&mut self) {
        self.jump_requested = true;
    }
}

impl Default for CharacterController {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn move_character_controllers(
    query: Query<(&mut CharacterController, &RigidBody, &Collider, &Transform)>,
    mut physics_state: ResMut<PhysicsState>,
) {
    let dt = Time::fixed_delta_time();

    for (mut controller, rigid_body, collider, transform) in query.iter() {
        if controller.grounded && controller.jump_requested {
            controller.vertical_velocity = controller.jump_speed;
        } else if !controller.grounded {
            controller.vertical_velocity -= controller.gravity * dt;
        } else if controller.vertical_velocity < 0.0 {
            controller.vertical_velocity = 0.0;
        }
        controller.jump_requested = false;

        let desired_translation =
            controller.desired_translation * dt + Vec3::Y * controller.vertical_velocity * dt;

        let Some(rapier_collider) = physics_state.collider_set.get(collider.0) else {
            continue;
        };
        let character_shape = rapier_collider.shape();
        let character_pos: Isometry<f32> = Isometry::from_parts(
            Translation::new(
                transform.translation.x,
                transform.translation.y,
                transform.translation.z,
            ),
            Rotation::new_unchecked(Quaternion::new(
                transform.rotation.w,
                transform.rotation.x,
                transform.rotation.y,
                transform.rotation.z,
            )),
        );
        let filter = QueryFilter::new().exclude_rigid_body(**rigid_body);

        let movement = controller.controller.move_shape(
            dt,
            &physics_state.rigid_body_set,
            &physics_state.collider_set,
            &physics_state.query_pipeline,
            character_shape,
            &character_pos,
            rapier3d::math::Vector::new(
                desired_translation.x,
                desired_translation.y,
                desired_translation.z,
            ),
            filter,
            |_collision| {},
        );

        controller.grounded = movement.grounded;
        if movement.grounded && controller.vertical_velocity < 0.0 {
            controller.vertical_velocity = 0.0;
        }

        let body = &mut physics_state.rigid_body_set[**rigid_body];
        let next_translation = *body.translation() + movement.translation;
        body.set_next_kinematic_translation(next_translation);
    }
}
