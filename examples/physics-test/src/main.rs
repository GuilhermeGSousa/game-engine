//! A simple, headless Jolt collision test.
//!
//! It builds a static floor and drops two dynamic spheres so you can see Jolt's
//! collision response at work:
//!   * the lower sphere falls and stops resting on the floor, and
//!   * the upper sphere falls and stops resting on the lower sphere.
//!
//! The world is advanced manually with [`PhysicsPipeline`], and each body's
//! height is printed over time, then checked against where it should settle.
//! No window or GPU is needed — run it with `cargo run -p physics-test`.

use essential::transform::Transform;
use glam::{Quat, Vec3};
use jolt_physics::physics_pipeline::PhysicsPipeline;
use jolt_physics::physics_state::PhysicsState;
use jolt_physics::rigid_body::RigidBody;

/// Simulation rate used for the manual stepping loop.
const FPS: f32 = 60.0;
/// Sphere radius shared by both dropped spheres.
const RADIUS: f32 = 1.0;

fn main() {
    let mut state = PhysicsState::new();
    let mut pipeline = PhysicsPipeline::new();

    // Static floor: a 50 x 0.5 x 50 (half-extent) box whose centre sits at
    // y = -0.5, so its top surface is exactly at y = 0.
    let floor_transform =
        Transform::from_translation_rotation(Vec3::new(0.0, -0.5, 0.0), Quat::IDENTITY);
    state.make_cuboid(50.0, 0.5, 50.0, &floor_transform, None);
    println!("floor: top surface at y = 0.0");

    // Lower sphere, dropped from y = 5. Should settle at y = RADIUS (resting on
    // the floor).
    let lower = spawn_sphere(&mut state, 5.0);
    // Upper sphere, dropped from y = 9. Should settle on top of the lower
    // sphere at y = 3 * RADIUS.
    let upper = spawn_sphere(&mut state, 9.0);

    println!("\nstepping {FPS} Hz for 3 s...\n");
    println!("{:>6} | {:>10} | {:>10}", "t (s)", "lower y", "upper y");
    println!("{:->6}-+-{:->10}-+-{:->10}", "", "", "");

    let steps = (FPS * 3.0) as u32;
    for step in 0..steps {
        pipeline.step(&mut state);

        // Print roughly every 0.25 s.
        if step % (FPS as u32 / 4) == 0 {
            let t = step as f32 / FPS;
            println!(
                "{:>6.2} | {:>10.3} | {:>10.3}",
                t,
                height(&state, &lower),
                height(&state, &upper),
            );
        }
    }

    let lower_y = height(&state, &lower);
    let upper_y = height(&state, &upper);

    let expected_lower = RADIUS; // resting on the floor
    let expected_upper = 3.0 * RADIUS; // resting on the lower sphere

    println!("\nresults:");
    report("lower sphere rests on floor", lower_y, expected_lower);
    report("upper sphere rests on lower sphere", upper_y, expected_upper);
}

/// Spawns a dynamic sphere centred on the Y axis at the given drop height.
fn spawn_sphere(state: &mut PhysicsState, drop_height: f32) -> RigidBody {
    let transform =
        Transform::from_translation_rotation(Vec3::new(0.0, drop_height, 0.0), Quat::IDENTITY);
    let body = RigidBody::new(&transform, state);
    state.make_sphere(&body, RADIUS);
    body
}

/// Reads a body's current Y position from the simulation.
fn height(state: &PhysicsState, body: &RigidBody) -> f32 {
    state.get_rigid_body(body).translation.y
}

/// Prints a pass/fail line comparing an observed resting height to the expected
/// one (within a small tolerance).
fn report(label: &str, actual: f32, expected: f32) {
    let ok = (actual - expected).abs() < 0.25;
    let status = if ok { "OK  " } else { "FAIL" };
    println!("  [{status}] {label}: y = {actual:.3} (expected ~{expected:.3})");
}
