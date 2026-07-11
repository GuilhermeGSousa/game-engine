use ecs::component::{Component, ComponentLifecycleCallback};
use essential::transform::Transform;
use glam::Vec3;

use crate::physics_state::PhysicsState;
use crate::rigid_body::RigidBody;

#[derive(Clone, Copy, Debug)]
pub enum Collider {
    Sphere {
        radius: f32,
    },
    Cuboid {
        half_extents: Vec3,
    },
    /// A capsule along the local Y axis with total height
    /// `2 * (half_height + radius)`: a cylinder of `2 * half_height` capped
    /// by hemispheres of `radius`.
    Capsule {
        half_height: f32,
        radius: f32,
    },
}

impl Collider {
    /// A sphere collider of the given radius.
    pub fn sphere(radius: f32) -> Self {
        Self::Sphere { radius }
    }

    /// A box collider (`width`/`height`/`length` are half-extents).
    pub fn cuboid(width: f32, height: f32, length: f32) -> Self {
        Self::Cuboid {
            half_extents: Vec3::new(width, height, length),
        }
    }

    /// A capsule collider (see [`Collider::Capsule`]).
    pub fn capsule(half_height: f32, radius: f32) -> Self {
        Self::Capsule {
            half_height,
            radius,
        }
    }
}

impl Component for Collider {
    fn name() -> &'static str {
        "Collider"
    }

    fn on_add() -> Option<ComponentLifecycleCallback> {
        Some(|mut world, context| {
            let collider = *world
                .get_component_for_entity::<Collider>(context.entity)
                .expect("on_add ran for an entity without a Collider");
            let transform = world
                .get_component_for_entity::<Transform>(context.entity)
                .cloned()
                .unwrap_or_default();
            let rigid_body = world
                .get_component_for_entity::<RigidBody>(context.entity)
                .copied();

            if let Some(state) = world.get_resource_mut::<PhysicsState>() {
                let body = state.create_body(collider, &transform, rigid_body);
                state.register_body_entity(body, context.entity);
            }
        })
    }

    fn on_remove() -> Option<ComponentLifecycleCallback> {
        Some(|mut world, context| {
            // The body id comes from the entity-to-body map, not the
            // component: `remove_component` fires this after the component is
            // already gone.
            if let Some(state) = world.get_resource_mut::<PhysicsState>() {
                if let Some(body) = state.get_body(context.entity) {
                    state.destroy_body(body);
                    state.unregister_body_entity(body, context.entity);
                }
            }
        })
    }
}
