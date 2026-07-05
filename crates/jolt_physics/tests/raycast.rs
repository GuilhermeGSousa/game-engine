//! Raycast smoke tests: rays against a static floor box report the correct
//! hit fraction, point, and surface normal, and rays that miss report nothing.

use essential::transform::Transform;
use glam::Vec3;
use jolt_physics::physics_state::PhysicsState;

#[test]
fn ray_hits_floor_from_above() {
    let mut state = PhysicsState::new();

    // Static floor: a 100 x 1 x 100 (half-extent) box centred at the origin,
    // so its top surface is at y = 1.
    let floor_transform = Transform::from_translation_rotation(Vec3::ZERO, Default::default());
    state.make_cuboid(100.0, 1.0, 100.0, &floor_transform, None);

    // Cast straight down from y = 5 with a reach of 10 units: the floor top at
    // y = 1 is 4 units away, so the hit fraction should be 0.4.
    let hit = state
        .cast_ray(Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.0, -10.0, 0.0))
        .expect("ray pointing at the floor should hit it");

    assert!(
        (hit.fraction - 0.4).abs() < 0.01,
        "expected hit fraction ~0.4, got {}",
        hit.fraction
    );
    assert!(
        (hit.point - Vec3::new(0.0, 1.0, 0.0)).length() < 0.05,
        "expected hit point near (0, 1, 0), got {}",
        hit.point
    );
    assert!(
        (hit.normal - Vec3::Y).length() < 0.01,
        "expected an upward surface normal, got {}",
        hit.normal
    );
}

#[test]
fn ray_misses_when_pointing_away() {
    let mut state = PhysicsState::new();

    let floor_transform = Transform::from_translation_rotation(Vec3::ZERO, Default::default());
    state.make_cuboid(100.0, 1.0, 100.0, &floor_transform, None);

    // Pointing up, away from the floor.
    let hit = state.cast_ray(Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.0, 10.0, 0.0));
    assert!(hit.is_none(), "upward ray should not hit the floor");

    // Pointing down but too short to reach it (floor top is 4 units away).
    let hit = state.cast_ray(Vec3::new(0.0, 5.0, 0.0), Vec3::new(0.0, -2.0, 0.0));
    assert!(hit.is_none(), "short ray should stop before the floor");
}
