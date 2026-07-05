//! The body-to-entity cache must track `RigidBody` component additions and
//! removals so `PhysicsState::get_entity` resolves raycast hits in O(1).

use ecs::world::World;
use essential::transform::Transform;
use glam::Vec3;
use jolt_physics::physics_state::PhysicsState;
use jolt_physics::rigid_body::RigidBody;

#[test]
fn cache_tracks_component_lifecycle() {
    let mut world = World::new();
    world.register_component_lifetimes::<RigidBody>();
    world.insert_resource(PhysicsState::new());

    let transform =
        Transform::from_translation_rotation(Vec3::new(0.0, 5.0, 0.0), Default::default());
    let state = world.get_resource_mut::<PhysicsState>().unwrap();
    let rigid_body = RigidBody::new(&transform, state);
    let body = *rigid_body;

    // Spawning the component registers the mapping. (The entity keeps its
    // Transform after the later removal: the ECS does not support removing an
    // entity's last remaining component.)
    let entity = world.spawn((rigid_body, transform));
    let state = world.get_resource::<PhysicsState>().unwrap();
    assert_eq!(
        state.get_entity(body),
        Some(entity),
        "spawn should register the body-to-entity mapping"
    );

    // Removing the component clears it.
    world.remove_component::<RigidBody>(entity);
    let state = world.get_resource::<PhysicsState>().unwrap();
    assert_eq!(
        state.get_entity(body),
        None,
        "removal should unregister the mapping"
    );
}

#[test]
fn cache_cleared_on_despawn() {
    let mut world = World::new();
    world.register_component_lifetimes::<RigidBody>();
    world.insert_resource(PhysicsState::new());

    let transform =
        Transform::from_translation_rotation(Vec3::new(0.0, 5.0, 0.0), Default::default());
    let state = world.get_resource_mut::<PhysicsState>().unwrap();
    let rigid_body = RigidBody::new(&transform, state);
    let body = *rigid_body;

    let entity = world.spawn((rigid_body,));
    world.despawn(entity);

    let state = world.get_resource::<PhysicsState>().unwrap();
    assert_eq!(
        state.get_entity(body),
        None,
        "despawn should unregister the mapping"
    );
}
