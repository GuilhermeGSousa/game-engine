//! `Collider`'s lifecycle must insert/remove the `BodyId` component and keep
//! the body-to-entity cache in sync so raycast hits resolve in O(1).

use ecs::world::World;
use essential::transform::Transform;
use glam::Vec3;
use physics::body::BodyId;
use physics::collider::Collider;
use physics::physics_state::PhysicsState;

fn physics_world() -> World {
    let mut world = World::new();
    world.register_component_lifetimes::<Collider>();
    world.insert_resource(PhysicsState::new());
    world
}
