use ecs::{CommandQueue, Component, component::bundle::ComponentBundle, entity::Entity};
use essential::transform::Transform;
use glam::{Quat, Vec3};
use physics::{
    character_controller::CharacterController, physics_state::PhysicsState, rigid_body::RigidBody,
};
use render::components::camera::Camera;

use crate::movement_state::CharacterMovementState;

#[derive(Component)]
pub struct Player;

/// Capsule collider dimensions for the third-person player: half-height of the cylindrical
/// section and radius, giving a ~2m tall capsule (matches an average human height).
pub const PLAYER_CAPSULE_RADIUS: f32 = 0.4;
pub const PLAYER_CAPSULE_HALF_HEIGHT: f32 = 0.6;

pub fn spawn_first_person_player<T: ComponentBundle + 'static>(
    cmd: &mut CommandQueue,
    pos: Vec3,
    extra_components: T,
) {
    let camera = Camera::default();

    cmd.spawn((
        Player,
        Transform::from_translation_rotation(pos, Quat::IDENTITY),
        extra_components,
    ))
    .add_child((
        camera,
        Transform::from_translation_rotation(Vec3::ZERO, Quat::IDENTITY),
    ));
}

/// Spawns a physics-driven third-person player: a kinematic capsule character controller.
/// Does not spawn a camera — pair with [`crate::camera::spawn_third_person_camera`].
pub fn spawn_third_person_player(
    cmd: &mut CommandQueue,
    physics_state: &mut PhysicsState,
    pos: Vec3,
) -> Entity {
    let transform = Transform::from_translation_rotation(pos, Quat::IDENTITY);
    let entity = *cmd.spawn((Player, transform.clone())).entity();

    let rigid_body = RigidBody::new_kinematic_position_based(&transform, physics_state);
    let collider = physics_state.make_capsule(
        entity,
        PLAYER_CAPSULE_RADIUS,
        PLAYER_CAPSULE_HALF_HEIGHT,
        &rigid_body,
    );

    cmd.insert(rigid_body, entity);
    cmd.insert(collider, entity);
    cmd.insert(CharacterController::new(), entity);
    cmd.insert(CharacterMovementState::default(), entity);

    entity
}
