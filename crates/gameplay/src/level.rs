use color::LinearRgba;
use ecs::{CommandQueue, component::Component, query::Query, resource::ResMut};
use essential::{assets::asset_server::AssetServer, time::Time, transform::Transform};
use glam::Vec3;
use mesh::MeshComponent;
use physics::{physics_state::PhysicsState, rigid_body::RigidBody};
use render::{assets::material::StandardMaterial, components::material::MaterialComponent};

use crate::primitives::make_box_mesh;

/// A kinematic platform that oscillates back and forth along `axis` around `center`.
///
/// Standing on it works "for free": rapier's `KinematicCharacterController` already transfers
/// a kinematic parent's velocity to a character resting on it (see
/// `detect_grounded_status_and_apply_friction` in rapier's character controller), so no extra
/// carry logic is needed here.
#[derive(Component)]
pub struct MovingPlatform {
    pub center: Vec3,
    pub axis: Vec3,
    pub amplitude: f32,
    pub period: f32,
    pub elapsed: f32,
}

/// Spawns a static platform: a visual box mesh plus a matching fixed box collider.
fn spawn_static_platform(
    cmd: &mut CommandQueue,
    asset_server: &AssetServer,
    physics_state: &mut PhysicsState,
    center: Vec3,
    half_extents: Vec3,
    color: LinearRgba,
) {
    let transform = Transform::from_translation(center);
    let entity = *cmd.spawn(transform.clone()).entity();

    let mesh_handle = asset_server.add(make_box_mesh(half_extents));
    let material_handle =
        asset_server.add(StandardMaterial::default().with_base_color_factor(color));

    let rigid_body = RigidBody::new_static(&transform, physics_state);
    let collider = physics_state.make_cuboid(
        entity,
        half_extents.x,
        half_extents.y,
        half_extents.z,
        &Transform::IDENTITY,
        Some(&rigid_body),
    );

    cmd.insert(
        MeshComponent {
            handle: mesh_handle,
        },
        entity,
    );
    cmd.insert(
        MaterialComponent::<StandardMaterial> {
            handle: material_handle,
        },
        entity,
    );
    cmd.insert(rigid_body, entity);
    cmd.insert(collider, entity);
}

/// Spawns a kinematic platform that oscillates along `axis` and carries the player like any
/// other piece of moving ground.
#[allow(clippy::too_many_arguments)]
fn spawn_moving_platform(
    cmd: &mut CommandQueue,
    asset_server: &AssetServer,
    physics_state: &mut PhysicsState,
    center: Vec3,
    half_extents: Vec3,
    axis: Vec3,
    amplitude: f32,
    period: f32,
    color: LinearRgba,
) {
    let transform = Transform::from_translation(center);
    let entity = *cmd.spawn(transform.clone()).entity();

    let mesh_handle = asset_server.add(make_box_mesh(half_extents));
    let material_handle =
        asset_server.add(StandardMaterial::default().with_base_color_factor(color));

    let rigid_body = RigidBody::new_kinematic_position_based(&transform, physics_state);
    let collider = physics_state.make_cuboid(
        entity,
        half_extents.x,
        half_extents.y,
        half_extents.z,
        &Transform::IDENTITY,
        Some(&rigid_body),
    );

    cmd.insert(
        MeshComponent {
            handle: mesh_handle,
        },
        entity,
    );
    cmd.insert(
        MaterialComponent::<StandardMaterial> {
            handle: material_handle,
        },
        entity,
    );
    cmd.insert(rigid_body, entity);
    cmd.insert(collider, entity);
    cmd.insert(
        MovingPlatform {
            center,
            axis: axis.normalize_or_zero(),
            amplitude,
            period,
            elapsed: 0.0,
        },
        entity,
    );
}

/// Spawns the tech demo's traversal gauntlet: a start pad, a basic gap jump, a raised ledge,
/// a moving-platform bridge, and a finish pad. All geometry is axis-aligned primitives, so it
/// only needs the box collider (no rotation support yet).
pub fn spawn_level(
    cmd: &mut CommandQueue,
    asset_server: &AssetServer,
    physics_state: &mut PhysicsState,
) {
    let ground_color = LinearRgba::new(0.55, 0.55, 0.6, 1.0);
    let ledge_color = LinearRgba::new(0.35, 0.55, 0.75, 1.0);
    let platform_color = LinearRgba::new(0.85, 0.55, 0.2, 1.0);
    let finish_color = LinearRgba::new(0.3, 0.8, 0.4, 1.0);

    spawn_static_platform(
        cmd,
        asset_server,
        physics_state,
        Vec3::new(0.0, 0.0, 0.0),
        Vec3::new(4.0, 0.5, 4.0),
        ground_color,
    );

    spawn_static_platform(
        cmd,
        asset_server,
        physics_state,
        Vec3::new(0.0, 0.0, -10.5),
        Vec3::new(3.0, 0.5, 3.0),
        ground_color,
    );

    spawn_static_platform(
        cmd,
        asset_server,
        physics_state,
        Vec3::new(0.0, 1.0, -19.0),
        Vec3::new(3.0, 0.5, 3.0),
        ledge_color,
    );

    spawn_moving_platform(
        cmd,
        asset_server,
        physics_state,
        Vec3::new(0.0, 1.0, -30.0),
        Vec3::new(2.0, 0.5, 2.0),
        Vec3::Z,
        4.0,
        4.0,
        platform_color,
    );

    spawn_static_platform(
        cmd,
        asset_server,
        physics_state,
        Vec3::new(0.0, 1.0, -42.0),
        Vec3::new(3.0, 0.5, 3.0),
        ground_color,
    );

    spawn_static_platform(
        cmd,
        asset_server,
        physics_state,
        Vec3::new(0.0, 1.0, -52.0),
        Vec3::new(4.0, 0.5, 4.0),
        finish_color,
    );
}

pub fn animate_moving_platforms(
    query: Query<(&mut MovingPlatform, &RigidBody)>,
    mut physics_state: ResMut<PhysicsState>,
) {
    let dt = Time::fixed_delta_time();

    for (mut platform, rigid_body) in query.iter() {
        platform.elapsed += dt;
        let phase = (platform.elapsed / platform.period) * std::f32::consts::TAU;
        let offset = platform.axis * (phase.sin() * platform.amplitude);
        let target = platform.center + offset;
        rigid_body.set_next_kinematic_translation(target, &mut physics_state);
    }
}
