//! The entity↔body maps must track `Collider` component additions and
//! removals so `PhysicsState::get_entity` (raycast hits) and
//! `PhysicsState::get_body` (simulation write-back) resolve in O(1).

use ecs::world::World;
use essential::transform::Transform;
use glam::Vec3;
use jolt_physics::collider::Collider;
use jolt_physics::physics_state::PhysicsState;

fn physics_world() -> World {
    let mut world = World::new();
    world.register_component_lifetimes::<Collider>();
    world.insert_resource(PhysicsState::new());
    world
}

#[test]
fn maps_track_component_lifecycle() {
    let mut world = physics_world();

    // Spawning the component creates the body and registers both directions.
    // (The entity keeps its Transform after the later removal: the ECS does
    // not support removing an entity's last remaining component.)
    let transform =
        Transform::from_translation_rotation(Vec3::new(0.0, 5.0, 0.0), Default::default());
    let entity = world.spawn((Collider::sphere(1.0), transform));

    let state = world.get_resource::<PhysicsState>().unwrap();
    let body = state
        .get_body(entity)
        .expect("spawn should create a body and register the entity-to-body mapping");
    assert_eq!(
        state.get_entity(body),
        Some(entity),
        "spawn should register the body-to-entity mapping"
    );

    // Removing the component clears both directions.
    world.remove_component::<Collider>(entity);
    let state = world.get_resource::<PhysicsState>().unwrap();
    assert_eq!(
        state.get_body(entity),
        None,
        "removal should unregister the entity-to-body mapping"
    );
    assert_eq!(
        state.get_entity(body),
        None,
        "removal should unregister the body-to-entity mapping"
    );
}

#[test]
fn maps_cleared_on_despawn() {
    let mut world = physics_world();

    let transform =
        Transform::from_translation_rotation(Vec3::new(0.0, 5.0, 0.0), Default::default());
    let entity = world.spawn((Collider::sphere(1.0), transform));

    let state = world.get_resource::<PhysicsState>().unwrap();
    let body = state.get_body(entity).expect("spawn should create a body");

    world.despawn(entity);

    let state = world.get_resource::<PhysicsState>().unwrap();
    assert_eq!(
        state.get_body(entity),
        None,
        "despawn should unregister the entity-to-body mapping"
    );
    assert_eq!(
        state.get_entity(body),
        None,
        "despawn should unregister the body-to-entity mapping"
    );
}
