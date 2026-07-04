use ecs::{CommandQueue, component::Component, entity::Entity, query::Query, resource::Res};
use essential::{time::Time, transform::Transform};
use glam::{Quat, Vec2, Vec3};
use physics::physics_state::PhysicsState;
use render::components::camera::Camera;
use window::input::Input;

/// A third-person follow camera that orbits a target entity, spring-collapsing its distance
/// when a raycast finds level geometry between the pivot and the desired camera position.
#[derive(Component)]
pub struct SpringArm {
    /// The entity to orbit and look at (its `Transform.translation`, plus `pivot_height`).
    pub target: Entity,
    pub yaw: f32,
    pub pitch: f32,
    /// The arm length the camera returns to when nothing is blocking it.
    pub desired_distance: f32,
    /// The arm length actually applied this frame, after collision collapse/spring.
    pub current_distance: f32,
    /// Minimum gap kept between the collision point and the camera, to avoid clipping into geometry.
    pub min_distance: f32,
    pub pitch_limits: (f32, f32),
    pub sensitivity: Vec2,
    /// How quickly `current_distance` springs back out toward `desired_distance` (per second).
    pub spring_speed: f32,
    /// Height above the target's origin that the camera orbits and looks at.
    pub pivot_height: f32,
}

impl SpringArm {
    pub fn new(target: Entity, desired_distance: f32) -> Self {
        Self {
            target,
            yaw: 0.0,
            pitch: -0.2,
            desired_distance,
            current_distance: desired_distance,
            min_distance: 0.3,
            pitch_limits: (-1.2, 0.9),
            sensitivity: Vec2::new(0.003, 0.003),
            spring_speed: 12.0,
            pivot_height: 1.6,
        }
    }

    /// The world-space rotation of the arm (and the camera, since the camera always looks at the pivot).
    pub fn orbit_rotation(&self) -> Quat {
        Quat::from_axis_angle(Vec3::Y, self.yaw) * Quat::from_axis_angle(Vec3::X, self.pitch)
    }
}

pub fn update_spring_arm_camera(
    spring_arms: Query<(&mut SpringArm, &mut Transform)>,
    targets: Query<&Transform>,
    physics_state: Res<PhysicsState>,
    input: Res<Input>,
    time: Res<Time>,
) {
    let dt = time.delta().as_secs_f32();

    for (mut arm, mut camera_transform) in spring_arms.iter() {
        let Some(target_transform) = targets.get_entity(arm.target) else {
            continue;
        };

        let mouse_delta = input.mouse_delta();
        arm.yaw -= mouse_delta.x * arm.sensitivity.x;
        arm.pitch = (arm.pitch - mouse_delta.y * arm.sensitivity.y)
            .clamp(arm.pitch_limits.0, arm.pitch_limits.1);

        let pivot = target_transform.translation + Vec3::Y * arm.pivot_height;
        let orbit_rotation = arm.orbit_rotation();
        let camera_backward = orbit_rotation * Vec3::Z;

        let hit =
            physics_state.cast_ray(pivot, camera_backward, arm.desired_distance, &[arm.target]);

        let target_distance = match hit {
            Some(hit) => (hit.distance - arm.min_distance).max(0.0),
            None => arm.desired_distance,
        };

        if target_distance < arm.current_distance {
            // Snap in immediately so the camera never clips through geometry for a frame.
            arm.current_distance = target_distance;
        } else {
            let t = 1.0 - (-arm.spring_speed * dt).exp();
            arm.current_distance += (target_distance - arm.current_distance) * t;
        }

        camera_transform.translation = pivot + camera_backward * arm.current_distance;
        camera_transform.rotation = orbit_rotation;
    }
}

/// Spawns a standalone camera entity (not parented to `target`) driven by a [`SpringArm`].
pub fn spawn_third_person_camera(
    cmd: &mut CommandQueue,
    target: Entity,
    desired_distance: f32,
) -> Entity {
    *cmd.spawn((
        Camera::default(),
        SpringArm::new(target, desired_distance),
        Transform::IDENTITY,
    ))
    .entity()
}
