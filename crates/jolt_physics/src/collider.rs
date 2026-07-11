use ecs::component::{Component, ComponentLifecycleCallback};
use essential::transform::Transform;
use glam::Vec3;

use crate::body::BodyId;
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

/// Local offset of the collider's geometry relative to the entity origin.
/// Optional sibling of [`Collider`], read when the body is created.
#[derive(Component, Clone, Copy, Debug)]
pub struct ColliderOffset(pub Vec3);

impl ColliderOffset {
    /// Lifts the shape so its bottom touches the entity origin — e.g. a
    /// standing character whose transform (and skeleton root) sits at the
    /// feet.
    pub fn bottom_origin(collider: &Collider) -> Self {
        let lift = match collider {
            Collider::Sphere { radius } => *radius,
            Collider::Cuboid { half_extents } => half_extents.y,
            Collider::Capsule {
                half_height,
                radius,
            } => half_height + radius,
        };
        Self(Vec3::Y * lift)
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
            let offset = world
                .get_component_for_entity::<ColliderOffset>(context.entity)
                .copied();

            let Some(state) = world.get_resource_mut::<PhysicsState>() else {
                return;
            };
            let body = state.create_body(collider, &transform, rigid_body, offset);
            state.register_body_entity(body, context.entity);
            world.insert_component(body, context.entity, false);
        })
    }

    fn on_remove() -> Option<ComponentLifecycleCallback> {
        Some(|mut world, context| {
            // `remove_component::<Collider>` fires this after the Collider is
            // gone; the body id survives on its own `BodyId` component.
            if let Some(&body) = world.get_component_for_entity::<BodyId>(context.entity) {
                if let Some(state) = world.get_resource_mut::<PhysicsState>() {
                    state.destroy_body(body);
                    state.unregister_body_entity(body);
                }
                world.remove_component::<BodyId>(context.entity, true);
            }
        })
    }
}
