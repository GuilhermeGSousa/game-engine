use std::ops::{Deref, DerefMut};

use ecs::component::{Component, ComponentLifecycleCallback};
use essential::transform::Transform;

use crate::body::BodyId;
use crate::physics_state::PhysicsState;

/// A dynamic rigid body. Wraps the Jolt [`BodyId`]; attach a shape to it with
/// [`PhysicsState::make_sphere`](crate::physics_state::PhysicsState::make_sphere)
/// or [`make_cuboid`](crate::physics_state::PhysicsState::make_cuboid).
pub struct RigidBody(BodyId);

// Implemented manually (instead of derived) for the lifecycle callbacks: they
// keep `PhysicsState`'s body-to-entity cache in sync so
// [`PhysicsState::get_entity`] is an O(1) lookup.
impl Component for RigidBody {
    fn name() -> &'static str {
        "RigidBody"
    }

    fn on_add() -> Option<ComponentLifecycleCallback> {
        Some(|mut world, context| {
            let body = **world
                .get_component_for_entity::<RigidBody>(context.entity)
                .expect("on_add ran for an entity without a RigidBody");
            if let Some(state) = world.get_resource_mut::<PhysicsState>() {
                state.register_body_entity(body, context.entity);
            }
        })
    }

    fn on_remove() -> Option<ComponentLifecycleCallback> {
        Some(|mut world, context| {
            // `despawn` fires this while the component is still readable
            // (fast path); `remove_component` fires it after the component is
            // gone, so fall back to evicting the cache entry by entity.
            let body = world
                .get_component_for_entity::<RigidBody>(context.entity)
                .map(|rigid_body| **rigid_body);
            if let Some(state) = world.get_resource_mut::<PhysicsState>() {
                match body {
                    Some(body) => state.unregister_body_entity(body),
                    None => state.unregister_entity(context.entity),
                }
            }
        })
    }
}

impl RigidBody {
    /// Creates a dynamic body at `transform`'s position and adds it to the
    /// simulation.
    ///
    /// Jolt requires a shape at body-creation time, so the body starts with a
    /// small placeholder sphere. A subsequent `make_sphere`/`make_cuboid` call
    /// replaces it with the real collider.
    pub fn new(transform: &Transform, state: &mut PhysicsState) -> Self {
        let position = transform.translation.to_array();

        // SAFETY: `state.world()` is a valid world, and `position` is a valid
        // xyz triple.
        let id =
            unsafe { jolt_ffi::jolt_body_create_dynamic(state.world(), position.as_ptr(), 0.5) };

        Self(BodyId(id))
    }
}

impl Deref for RigidBody {
    type Target = BodyId;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RigidBody {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
