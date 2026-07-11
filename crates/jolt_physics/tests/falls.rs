//! End-to-end smoke test: a dynamic sphere dropped above a static floor should
//! fall under gravity and come to rest on top of the floor. Bodies are created
//! by spawning `Collider` components — never directly.

use ecs::world::World;
use essential::transform::Transform;
use glam::{Quat, Vec3};
use jolt_physics::collider::Collider;
use jolt_physics::physics_pipeline::PhysicsPipeline;
use jolt_physics::physics_state::PhysicsState;
use jolt_physics::rigid_body::{AllowedDofs, RigidBody};

#[test]
fn sphere_falls_and_rests_on_floor() {
    let mut world = World::new();
    world.register_component_lifetimes::<Collider>();
    world.insert_resource(PhysicsState::new());
    let mut pipeline = PhysicsPipeline::new();

    // Static floor: a 100 x 1 x 100 (half-extent) box centred at the origin, so
    // its top surface is at y = 1.
    world.spawn((
        Collider::cuboid(100.0, 1.0, 100.0),
        Transform::from_translation_rotation(Vec3::ZERO, Default::default()),
    ));

    // Dynamic sphere of radius 1, dropped from y = 10.
    let sphere = world.spawn((
        RigidBody::default(),
        Collider::sphere(1.0),
        Transform::from_translation_rotation(Vec3::new(0.0, 10.0, 0.0), Default::default()),
    ));

    let state = world.get_resource::<PhysicsState>().unwrap();
    let body = state
        .get_body(sphere)
        .expect("spawning a Collider should create a body");
    let start_y = state.body_transform(body).translation.y;
    assert!(
        (start_y - 10.0).abs() < 0.5,
        "sphere should start near y = 10, was {start_y}"
    );

    // Step ~3 seconds at 60 Hz.
    for _ in 0..180 {
        pipeline.step(world.get_resource_mut::<PhysicsState>().unwrap());
    }

    let end_y = world
        .get_resource::<PhysicsState>()
        .unwrap()
        .body_transform(body)
        .translation
        .y;

    // It must have fallen substantially...
    assert!(
        end_y < start_y - 5.0,
        "sphere should have fallen, y = {end_y}"
    );
    // ...and settled near the floor top (y = 1) + sphere radius (1) = 2.
    assert!(
        (end_y - 2.0).abs() < 0.5,
        "sphere should rest near y = 2 (floor top + radius), was {end_y}"
    );
}

/// The player-capsule setup: a capsule with the rotation DOFs locked must
/// land upright and stay upright.
#[test]
fn capsule_with_locked_rotation_rests_upright() {
    let mut world = World::new();
    world.register_component_lifetimes::<Collider>();
    world.insert_resource(PhysicsState::new());
    let mut pipeline = PhysicsPipeline::new();

    // Floor top at y = 1.
    world.spawn((
        Collider::cuboid(100.0, 1.0, 100.0),
        Transform::from_translation_rotation(Vec3::ZERO, Default::default()),
    ));

    // Capsule of total half-height 1 (cylinder half-height 0.5 + cap radius
    // 0.5), dropped from y = 5.
    let capsule = world.spawn((
        RigidBody {
            allowed_dofs: AllowedDofs::TRANSLATION,
            ..Default::default()
        },
        Collider::capsule(0.5, 0.5),
        Transform::from_translation_rotation(Vec3::new(0.0, 5.0, 0.0), Default::default()),
    ));

    for _ in 0..180 {
        pipeline.step(world.get_resource_mut::<PhysicsState>().unwrap());
    }

    let state = world.get_resource::<PhysicsState>().unwrap();
    let body = state.get_body(capsule).expect("capsule should have a body");
    let transform = state.body_transform(body);

    // Rest height: floor top (1) + capsule half-height (1) = 2.
    assert!(
        (transform.translation.y - 2.0).abs() < 0.5,
        "capsule should rest near y = 2 (floor top + half-height), was {}",
        transform.translation.y
    );
    assert!(
        transform.rotation.dot(Quat::IDENTITY).abs() > 0.999,
        "rotation-locked capsule should stay upright, was {:?}",
        transform.rotation
    );
}
