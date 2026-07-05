use std::collections::HashMap;

use ecs::entity::Entity;
use ecs::resource::Resource;
use essential::transform::Transform;
use glam::{Quat, Vec3};

use crate::body::BodyId;
use crate::collider::Collider;
use crate::ray::RayHit;
use crate::rigid_body::RigidBody;

const MAX_BODIES: u32 = 10_240;
const NUM_BODY_MUTEXES: u32 = 0; // 0 = let Jolt pick a default
                                 // These bound the per-step scratch Jolt allocates from the temp allocator
                                 // (see `PhysicsPipeline`). Keep them in step with that allocator's size.
const MAX_BODY_PAIRS: u32 = 10_240;
const MAX_CONTACT_CONSTRAINTS: u32 = 10_240;

/// Owns the Jolt physics world: the body store, broad/narrow phases, and the
/// collision-layer interfaces it was initialised with (all held together on
/// the C++ side of the `jolt-sys` shim).
#[derive(Resource)]
pub struct PhysicsState {
    world: *mut jolt_sys::JoltWorld,
    /// Maps Jolt body ids to the entity holding the matching [`RigidBody`]
    /// component. Kept in sync by `RigidBody`'s lifecycle callbacks so
    /// body-to-entity lookups (e.g. after a raycast) are O(1).
    body_to_entity: HashMap<jolt_sys::JoltBodyId, Entity>,
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
            jolt_sys::jolt_world_create(
                MAX_BODIES,
                NUM_BODY_MUTEXES,
                MAX_BODY_PAIRS,
                MAX_CONTACT_CONSTRAINTS,
            )
        };

        Self {
            world,
            body_to_entity: HashMap::new(),
        }
    }

    /// The entity whose [`RigidBody`] owns `body`, if any.
    pub fn get_entity(&self, body: BodyId) -> Option<Entity> {
        self.body_to_entity.get(&body.0).copied()
    }

    pub(crate) fn register_body_entity(&mut self, body: BodyId, entity: Entity) {
        self.body_to_entity.insert(body.0, entity);
    }

    pub(crate) fn unregister_body_entity(&mut self, body: BodyId) {
        self.body_to_entity.remove(&body.0);
    }

    /// Evicts a cache entry by entity, for removal paths where the
    /// `RigidBody` component (and thus its body id) is no longer readable.
    pub(crate) fn unregister_entity(&mut self, entity: Entity) {
        self.body_to_entity.retain(|_, e| *e != entity);
    }

    /// The raw Jolt world pointer, used by [`PhysicsPipeline`] to drive
    /// stepping. The pointer borrows from `self` and must not outlive it.
    ///
    /// [`PhysicsPipeline`]: crate::physics_pipeline::PhysicsPipeline
    pub(crate) fn world(&self) -> *mut jolt_sys::JoltWorld {
        self.world
    }

    /// Replaces a body's shape with a sphere of the given radius.
    pub fn make_sphere(&mut self, parent: &RigidBody, radius: f32) -> Collider {
        let body = **parent; // BodyId (Copy) via Deref

        // SAFETY: `body` refers to a body that was added to this world.
        unsafe {
            jolt_sys::jolt_body_set_sphere_shape(self.world, body.0, radius);
        }

        Collider(body)
    }

    /// Builds a box collider (`width`/`height`/`length` are half-extents).
    ///
    /// With a `parent`, the box replaces that dynamic body's shape. Without one,
    /// a new *static* body is created at `transform`'s position to hold the box
    /// (used for level geometry such as floors).
    pub fn make_cuboid(
        &mut self,
        width: f32,
        height: f32,
        length: f32,
        transform: &Transform,
        parent: Option<&RigidBody>,
    ) -> Collider {
        let half_extents = [width, height, length];

        match parent {
            Some(rb) => {
                let body = **rb; // BodyId (Copy) via Deref

                // SAFETY: `body` is a body in this world; the half-extents
                // array is a valid xyz triple.
                unsafe {
                    jolt_sys::jolt_body_set_box_shape(self.world, body.0, half_extents.as_ptr());
                }
                Collider(body)
            }
            None => {
                let position = transform.translation.to_array();

                // SAFETY: both arrays are valid xyz triples.
                let id = unsafe {
                    jolt_sys::jolt_body_create_static_box(
                        self.world,
                        position.as_ptr(),
                        half_extents.as_ptr(),
                    )
                };

                Collider(BodyId(id))
            }
        }
    }

    /// Reads a body's current world transform out of the simulation.
    pub fn get_rigid_body(&self, rigid_body: &RigidBody) -> Transform {
        let body = **rigid_body; // BodyId (Copy) via Deref

        let mut position = [0.0f32; 3];
        let mut rotation = [0.0f32; 4];
        // SAFETY: `body` is a valid body id within this world, and the output
        // buffers have the sizes the shim writes (xyz and xyzw).
        unsafe {
            jolt_sys::jolt_body_get_transform(
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
        let mut hit = jolt_sys::JoltRayHit {
            body: 0,
            fraction: 0.0,
            normal: [0.0; 3],
        };

        // SAFETY: the input arrays are valid xyz triples and `hit` is a valid
        // out-buffer; the shim only writes it when returning true.
        let did_hit = unsafe {
            jolt_sys::jolt_world_cast_ray(
                self.world,
                origin_array.as_ptr(),
                direction_array.as_ptr(),
                &mut hit,
            )
        };

        did_hit
            .then(|| {
                let body = BodyId(hit.body);

                if let Some(hit_entity) = self.get_entity(body) {
                    Some(RayHit {
                        body,
                        entity: hit_entity,
                        fraction: hit.fraction,
                        point: origin + direction * hit.fraction,
                        normal: Vec3::from(hit.normal),
                    })
                } else {
                    None
                }
            })
            .flatten()
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
            jolt_sys::jolt_world_destroy(self.world);
        }
    }
}
