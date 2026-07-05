//! End-to-end smoke test: a dynamic sphere dropped above a static floor should
//! fall under gravity and come to rest on top of the floor.

use essential::transform::Transform;
use glam::Vec3;
use jolt_physics::physics_pipeline::PhysicsPipeline;
use jolt_physics::physics_state::PhysicsState;
use jolt_physics::rigid_body::RigidBody;

#[test]
fn sphere_falls_and_rests_on_floor() {
    let mut state = PhysicsState::new();
    let mut pipeline = PhysicsPipeline::new();

    // Static floor: a 100 x 1 x 100 (half-extent) box centred at the origin, so
    // its top surface is at y = 1.
    let floor_transform = Transform::from_translation_rotation(Vec3::ZERO, Default::default());
    state.make_cuboid(100.0, 1.0, 100.0, &floor_transform, None);

    // Dynamic sphere of radius 1, dropped from y = 10.
    let start = Transform::from_translation_rotation(Vec3::new(0.0, 10.0, 0.0), Default::default());
    let body = RigidBody::new(&start, &mut state);
    state.make_sphere(&body, 1.0);

    let start_y = state.get_rigid_body(&body).translation.y;
    assert!(
        (start_y - 10.0).abs() < 0.5,
        "sphere should start near y = 10, was {start_y}"
    );

    // Step ~3 seconds at 60 Hz.
    for _ in 0..180 {
        pipeline.step(&mut state);
    }

    let end_y = state.get_rigid_body(&body).translation.y;

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
