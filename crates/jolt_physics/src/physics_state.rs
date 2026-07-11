use std::collections::HashMap;

use ecs::entity::Entity;
use ecs::resource::Resource;
use essential::transform::Transform;
use glam::{Quat, Vec3};

use crate::body::BodyId;
use crate::collider::ColliderShape;
use crate::ray::RayHit;

const MAX_BODIES: u32 = 10_240;
const NUM_BODY_MUTEXES: u32 = 0; // 0 = let Jolt pick a default
                                 // These bound the per-step scratch Jolt allocates from the temp allocator
                                 // (see `PhysicsPipeline`). Keep them in step with that allocator's size.
const MAX_BODY_PAIRS: u32 = 10_240;
const MAX_CONTACT_CONSTRAINTS: u32 = 10_240;

/// Owns the Jolt physics world: the body store, broad/narrow phases, and the
/// collision-layer interfaces it was initialised with (all held together on
/// the C++ side of the `jolt-ffi` shim).
#[derive(Resource)]
pub struct PhysicsState {
    world: *mut jolt_ffi::JoltWorld,
    /// Maps Jolt body ids to the entity holding the matching [`Collider`]
    /// component, and back. Kept in sync by `Collider`'s lifecycle callbacks
    /// so body-to-entity lookups (e.g. after a raycast) and entity-to-body
    /// lookups (e.g. in the simulation write-back) are O(1).
    ///
    /// [`Collider`]: crate::collider::Collider
    body_to_entity: HashMap<jolt_ffi::JoltBodyId, Entity>,
    entity_to_body: HashMap<Entity, BodyId>,
}

// SAFETY: `PhysicsState` holds a raw pointer into Jolt, which is not inherently
// `Send`/`Sync`. The ECS requires `Resource: Send + Sync`, and the scheduler
// upholds the actual safety contract: a `ResMut<PhysicsState>` is granted
// exclusive access, so the world is never touched from two threads at once.
unsafe impl Send for PhysicsState {}
unsafe impl Sync for PhysicsState {}

impl PhysicsState {
    pub fn new() -> Self {
        // SAFETY: `jolt_world_create` performs Jolt's process-global init
        // (idempotent) and returns an owned world, freed in `Drop`.
        let world = unsafe {
            jolt_ffi::jolt_world_create(
                MAX_BODIES,
                NUM_BODY_MUTEXES,
                MAX_BODY_PAIRS,
                MAX_CONTACT_CONSTRAINTS,
            )
        };

        Self {
            world,
            body_to_entity: HashMap::new(),
            entity_to_body: HashMap::new(),
        }
    }

    /// The entity whose [`Collider`](crate::collider::Collider) owns `body`,
    /// if any.
    pub fn get_entity(&self, body: BodyId) -> Option<Entity> {
        self.body_to_entity.get(&body.0).copied()
    }

    /// The Jolt body backing `entity`'s
    /// [`Collider`](crate::collider::Collider), if any.
    pub fn get_body(&self, entity: Entity) -> Option<BodyId> {
        self.entity_to_body.get(&entity).copied()
    }

    pub(crate) fn register_body_entity(&mut self, body: BodyId, entity: Entity) {
        self.body_to_entity.insert(body.0, entity);
        self.entity_to_body.insert(entity, body);
    }

    pub(crate) fn unregister_body_entity(&mut self, body: BodyId, entity: Entity) {
        self.body_to_entity.remove(&body.0);
        self.entity_to_body.remove(&entity);
    }

    /// The raw Jolt world pointer, used by [`PhysicsPipeline`] to drive
    /// stepping. The pointer borrows from `self` and must not outlive it.
    ///
    /// [`PhysicsPipeline`]: crate::physics_pipeline::PhysicsPipeline
    pub(crate) fn world(&self) -> *mut jolt_ffi::JoltWorld {
        self.world
    }

    /// Creates a body with the given shape at `transform`'s position and adds
    /// it to the simulation: dynamic with the given density when `density` is
    /// `Some`, static otherwise. Called by `Collider::on_add`.
    pub(crate) fn create_body(
        &mut self,
        shape: ColliderShape,
        transform: &Transform,
        density: Option<f32>,
    ) -> BodyId {
        let position = transform.translation.to_array();

        // SAFETY: `self.world` is a valid world, and the position/half-extent
        // arrays are valid xyz triples.
        let id = unsafe {
            match (shape, density) {
                (ColliderShape::Sphere { radius }, Some(density)) => {
                    jolt_ffi::jolt_body_create_dynamic_sphere(
                        self.world,
                        position.as_ptr(),
                        radius,
                        density,
                    )
                }
                (ColliderShape::Sphere { radius }, None) => {
                    jolt_ffi::jolt_body_create_static_sphere(self.world, position.as_ptr(), radius)
                }
                (ColliderShape::Cuboid { half_extents }, Some(density)) => {
                    jolt_ffi::jolt_body_create_dynamic_box(
                        self.world,
                        position.as_ptr(),
                        half_extents.to_array().as_ptr(),
                        density,
                    )
                }
                (ColliderShape::Cuboid { half_extents }, None) => {
                    jolt_ffi::jolt_body_create_static_box(
                        self.world,
                        position.as_ptr(),
                        half_extents.to_array().as_ptr(),
                    )
                }
            }
        };

        BodyId(id)
    }

    /// Removes `body` from the simulation and destroys it. Called by
    /// `Collider::on_remove`.
    pub(crate) fn destroy_body(&mut self, body: BodyId) {
        // SAFETY: `body` refers to a body created in this world and not yet
        // destroyed (`Collider`'s lifecycle guarantees one destroy per create).
        unsafe {
            jolt_ffi::jolt_body_destroy(self.world, body.0);
        }
    }

    /// Reads a body's current world transform out of the simulation.
    pub fn body_transform(&self, body: BodyId) -> Transform {
        let mut position = [0.0f32; 3];
        let mut rotation = [0.0f32; 4];
        // SAFETY: `body` is a valid body id within this world, and the output
        // buffers have the sizes the shim writes (xyz and xyzw).
        unsafe {
            jolt_ffi::jolt_body_get_transform(
                self.world,
                body.0,
                position.as_mut_ptr(),
                rotation.as_mut_ptr(),
            );
        }

        Transform::from_translation_rotation(Vec3::from(position), Quat::from_array(rotation))
    }

    /// Casts a ray from `origin` along `direction` — whose length is the
    /// maximum cast distance — and returns the closest hit, if any.
    pub fn cast_ray(&self, origin: Vec3, direction: Vec3) -> Option<RayHit> {
        let origin_array = origin.to_array();
        let direction_array = direction.to_array();
        let mut hit = jolt_ffi::JoltRayHit {
            body: 0,
            fraction: 0.0,
            normal: [0.0; 3],
        };

        // SAFETY: the input arrays are valid xyz triples and `hit` is a valid
        // out-buffer; the shim only writes it when returning true.
        let did_hit = unsafe {
            jolt_ffi::jolt_world_cast_ray(
                self.world,
                origin_array.as_ptr(),
                direction_array.as_ptr(),
                &mut hit,
            )
        };

        did_hit.then(|| {
            let body = BodyId(hit.body);

            RayHit {
                body,
                entity: self.get_entity(body),
                fraction: hit.fraction,
                point: origin + direction * hit.fraction,
                normal: Vec3::from(hit.normal),
            }
        })
    }
}

impl Default for PhysicsState {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for PhysicsState {
    fn drop(&mut self) {
        // SAFETY: `self.world` was created in `new` and is destroyed exactly
        // once here.
        unsafe {
            jolt_ffi::jolt_world_destroy(self.world);
        }
    }
}
