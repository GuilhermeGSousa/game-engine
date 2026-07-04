use ecs::{Query, Res, With};
use essential::{time::Time, transform::Transform};
use glam::{Quat, Vec2, Vec3};
use physics::character_controller::CharacterController;
use render::components::camera::Camera;
use window::input::{Input, InputState, KeyCode, PhysicalKey};

use crate::{camera::SpringArm, player::Player};

/// Walk speed, in world units/second.
pub const WALK_SPEED: f32 = 2.5;
/// Run speed, in world units/second. Also used to normalize speed for the locomotion blend space.
pub const RUN_SPEED: f32 = 6.0;
/// Maximum yaw turn rate when facing toward the movement direction, in radians/second.
const FACE_TURN_RATE: f32 = 720.0_f32.to_radians();

pub fn first_person_player_fly(
    players: Query<&mut Transform, With<Player>>,
    cameras: Query<&mut Transform, With<Camera>>,
    input: Res<Input>,
    time: Res<Time>,
) {
    let Some(mut player_transform) = players.iter().next() else {
        return;
    };

    let Some(mut camera_transform) = cameras.iter().next() else {
        return;
    };

    let displacement = 10.0 * time.delta().as_secs_f32();

    let key_d = input.get_key_state(PhysicalKey::Code(KeyCode::KeyD));
    let key_a = input.get_key_state(PhysicalKey::Code(KeyCode::KeyA));
    let key_w = input.get_key_state(PhysicalKey::Code(KeyCode::KeyW));
    let key_s = input.get_key_state(PhysicalKey::Code(KeyCode::KeyS));

    // Camera transform is local to the player, so combine rotations to get world-space directions.
    let camera_world_rotation = player_transform.rotation * camera_transform.rotation;
    let forward = camera_world_rotation * Vec3::NEG_Z;
    let back = -forward;
    let right = camera_world_rotation * Vec3::X;
    let left = -right;

    if key_d == InputState::Pressed || key_d == InputState::Down {
        player_transform.translation += right * displacement;
    }
    if key_a == InputState::Pressed || key_a == InputState::Down {
        player_transform.translation += left * displacement;
    }
    if key_w == InputState::Pressed || key_w == InputState::Down {
        player_transform.translation += forward * displacement;
    }
    if key_s == InputState::Pressed || key_s == InputState::Down {
        player_transform.translation += back * displacement;
    }

    let sensitivity = -0.003;
    let mouse_delta = input.mouse_delta();
    let yaw_delta = sensitivity * mouse_delta.x;
    let pitch_delta = sensitivity * mouse_delta.y;
    player_transform.rotation *= Quat::from_axis_angle(Vec3::Y, yaw_delta);
    camera_transform.rotation *= Quat::from_axis_angle(Vec3::X, pitch_delta);
}

/// Reads camera-relative WASD input and writes it into the player's [`CharacterController`].
///
/// Input is resolved against the spring arm's yaw (not the player's own facing), which is what
/// makes this "third-person, camera-relative" controls: pressing "forward" always moves the
/// character away from the camera, regardless of which way the character itself is currently
/// facing. [`rotate_player_to_face_movement`] then turns the character to face this direction.
pub fn third_person_movement_input(
    controllers: Query<&mut CharacterController, With<Player>>,
    spring_arms: Query<&SpringArm>,
    input: Res<Input>,
) {
    let Some(spring_arm) = spring_arms.iter().next() else {
        return;
    };
    let yaw = spring_arm.yaw;

    let camera_forward = Vec3::new(-yaw.sin(), 0.0, -yaw.cos());
    let camera_right = Vec3::new(yaw.cos(), 0.0, -yaw.sin());

    let mut input_dir = Vec2::ZERO;
    if input.is_held(PhysicalKey::Code(KeyCode::KeyW)) {
        input_dir.y += 1.0;
    }
    if input.is_held(PhysicalKey::Code(KeyCode::KeyS)) {
        input_dir.y -= 1.0;
    }
    if input.is_held(PhysicalKey::Code(KeyCode::KeyD)) {
        input_dir.x += 1.0;
    }
    if input.is_held(PhysicalKey::Code(KeyCode::KeyA)) {
        input_dir.x -= 1.0;
    }

    let world_dir = (camera_forward * input_dir.y + camera_right * input_dir.x).normalize_or_zero();
    let speed = if input.is_held(PhysicalKey::Code(KeyCode::ShiftLeft)) {
        RUN_SPEED
    } else {
        WALK_SPEED
    };

    for mut controller in controllers.iter() {
        controller.desired_translation = world_dir * speed;
    }
}

/// Turns the player to face its current movement direction, independently of camera yaw.
pub fn rotate_player_to_face_movement(
    query: Query<(&mut Transform, &CharacterController), With<Player>>,
    time: Res<Time>,
) {
    let dt = time.delta().as_secs_f32();

    for (mut transform, controller) in query.iter() {
        let dir = controller.desired_translation;
        if dir.length_squared() < 1.0e-6 {
            continue;
        }

        let target_yaw = (-dir.x).atan2(-dir.z);
        let target_rotation = Quat::from_rotation_y(target_yaw);

        let angle = transform.rotation.angle_between(target_rotation);
        if angle <= f32::EPSILON {
            continue;
        }

        let t = (FACE_TURN_RATE * dt / angle).min(1.0);
        transform.rotation = transform.rotation.slerp(target_rotation, t);
    }
}
