//! `Collider`'s lifecycle must insert/remove the `BodyId` component and keep
//! the body-to-entity cache in sync so raycast hits resolve in O(1).

use ecs::world::World;
use essential::transform::Transform;
use glam::Vec3;
use jolt_physics::body::BodyId;
use jolt_physics::collider::Collider;
use jolt_physics::physics_state::PhysicsState;

fn physics_world() -> World {
    let mut world = World::new();
    world.register_component_lifetimes::<Collider>();
    world.insert_resource(PhysicsState::new());
    world
}

#[test]
fn body_id_tracks_component_lifecycle() {
    let mut world = physics_world();

    // (The entity keeps its Transform after the later removal: the ECS does
    // not support removing an entity's last remaining component.)
    let transform =
        Transform::from_translation_rotation(Vec3::new(0.0, 5.0, 0.0), Default::default());
    let entity = world.spawn((Collider::sphere(1.0), transform));

    let body = *world
        .get_component_for_entity::<BodyId>(entity)
        .expect("spawn should insert the BodyId component");
    let state = world.get_resource::<PhysicsState>().unwrap();
    assert_eq!(
        state.get_entity(body),
        Some(entity),
        "spawn should register the body-to-entity mapping"
    );

    world.remove_component::<Collider>(entity);
    assert!(
        world.get_component_for_entity::<BodyId>(entity).is_none(),
        "removal should remove the BodyId component"
    );
    let state = world.get_resource::<PhysicsState>().unwrap();
    assert_eq!(
        state.get_entity(body),
        None,
        "removal should unregister the body-to-entity mapping"
    );
}

#[test]
fn cache_cleared_on_despawn() {
    let mut world = physics_world();

    let transform =
        Transform::from_translation_rotation(Vec3::new(0.0, 5.0, 0.0), Default::default());
    let entity = world.spawn((Collider::sphere(1.0), transform));

    let body = *world
        .get_component_for_entity::<BodyId>(entity)
        .expect("spawn should insert the BodyId component");

    world.despawn(entity);

    let state = world.get_resource::<PhysicsState>().unwrap();
    assert_eq!(
        state.get_entity(body),
        None,
        "despawn should unregister the body-to-entity mapping"
    );
}
