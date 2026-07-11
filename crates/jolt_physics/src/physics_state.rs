use std::collections::HashMap;

use ecs::entity::Entity;
use ecs::resource::Resource;
use essential::transform::Transform;
use glam::{Quat, Vec3};

use crate::body::BodyId;
use crate::collider::Collider;
use crate::ground::{GroundContact, GroundState};
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
        }
    }

    /// The entity whose [`Collider`](crate::collider::Collider) owns `body`,
    /// if any.
    pub fn get_entity(&self, body: BodyId) -> Option<Entity> {
        self.body_to_entity.get(&body.0).copied()
    }

    pub(crate) fn register_body_entity(&mut self, body: BodyId, entity: Entity) {
        self.body_to_entity.insert(body.0, entity);
    }

    pub(crate) fn unregister_body_entity(&mut self, body: BodyId) {
        self.body_to_entity.remove(&body.0);
    }

    /// The raw Jolt world pointer, used by [`PhysicsPipeline`] to drive
    /// stepping. The pointer borrows from `self` and must not outlive it.
    ///
    /// [`PhysicsPipeline`]: crate::physics_pipeline::PhysicsPipeline
    pub(crate) fn world(&self) -> *mut jolt_ffi::JoltWorld {
        self.world
    }

    /// Creates a body with the given shape at `transform`'s position and
    /// rotation and adds it to the simulation: dynamic with `rigid_body`'s
    /// parameters when it is `Some`, static otherwise. Called by
    /// `Collider::on_add`.
    pub(crate) fn create_body(
        &mut self,
        collider: Collider,
        transform: &Transform,
        rigid_body: Option<RigidBody>,
    ) -> BodyId {
        let position = transform.translation.to_array();
        let rotation = transform.rotation.to_array();
        let motion_type = match rigid_body {
            Some(rb) => match rb.motion_type {
                crate::rigid_body::MotionType::Dynamic => jolt_ffi::JOLT_MOTION_TYPE_DYNAMIC,
                crate::rigid_body::MotionType::Kinematic => jolt_ffi::JOLT_MOTION_TYPE_KINEMATIC,
            },
            None => jolt_ffi::JOLT_MOTION_TYPE_STATIC,
        };
        // Density is unused for static bodies; any sane value works here.
        let density = rigid_body.map_or(1000.0, |rigid_body| rigid_body.density);

        // SAFETY: `self.world` is a valid world; the position/half-extent
        // arrays are valid xyz triples and the rotation a valid xyzw
        // quaternion. The settings are created, used, and destroyed within
        // this call.
        let id = unsafe {
            let settings = jolt_ffi::jolt_body_creation_settings_create();
            jolt_ffi::jolt_body_creation_settings_set_position(settings, position.as_ptr());
            jolt_ffi::jolt_body_creation_settings_set_rotation(settings, rotation.as_ptr());
            jolt_ffi::jolt_body_creation_settings_set_motion_type(settings, motion_type);
            if let Some(rigid_body) = rigid_body {
                jolt_ffi::jolt_body_creation_settings_set_allowed_dofs(
                    settings,
                    rigid_body.allowed_dofs.0,
                );
            }
            match collider {
                Collider::Sphere { radius } => {
                    jolt_ffi::jolt_body_creation_settings_set_sphere_shape(
                        settings, radius, density,
                    );
                }
                Collider::Cuboid { half_extents } => {
                    jolt_ffi::jolt_body_creation_settings_set_box_shape(
                        settings,
                        half_extents.to_array().as_ptr(),
                        density,
                    );
                }
                Collider::Capsule {
                    half_height,
                    radius,
                } => {
                    jolt_ffi::jolt_body_creation_settings_set_capsule_shape(
                        settings,
                        half_height,
                        radius,
                        density,
                    );
                }
            }
            let id = jolt_ffi::jolt_body_create(self.world, settings);
            jolt_ffi::jolt_body_creation_settings_destroy(settings);
            id
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

    /// Reports what `body` is standing on: the most upward-facing contact
    /// within `max_separation` below its shape, classified against
    /// `max_slope_angle` (radians from horizontal).
    pub fn probe_ground(
        &self,
        body: BodyId,
        max_separation: f32,
        max_slope_angle: f32,
    ) -> GroundState {
        let mut result = jolt_ffi::JoltGroundProbeResult {
            state: jolt_ffi::JOLT_GROUND_STATE_IN_AIR,
            body: 0,
            position: [0.0; 3],
            normal: [0.0; 3],
            velocity: [0.0; 3],
        };
        // SAFETY: `body` is a body in this world and `result` is a valid
        // out-buffer.
        unsafe {
            jolt_ffi::jolt_body_probe_ground(
                self.world,
                body.0,
                max_separation,
                max_slope_angle,
                &mut result,
            );
        }

        let contact = || GroundContact {
            entity: self.get_entity(BodyId(result.body)),
            point: Vec3::from(result.position),
            normal: Vec3::from(result.normal),
            velocity: Vec3::from(result.velocity),
        };
        match result.state {
            jolt_ffi::JOLT_GROUND_STATE_ON_GROUND => GroundState::OnGround(contact()),
            jolt_ffi::JOLT_GROUND_STATE_ON_STEEP_GROUND => GroundState::OnSteepGround(contact()),
            _ => GroundState::InAir,
        }
    }

    pub fn set_linear_velocity(&mut self, body: BodyId, velocity: Vec3) {
        let velocity = velocity.to_array();
        // SAFETY: `body` is a body in this world; `velocity` is a valid xyz
        // triple.
        unsafe {
            jolt_ffi::jolt_body_set_linear_velocity(self.world, body.0, velocity.as_ptr());
        }
    }

    pub fn linear_velocity(&self, body: BodyId) -> Vec3 {
        let mut velocity = [0.0f32; 3];
        // SAFETY: `body` is a body in this world and the out-buffer is a
        // valid xyz triple.
        unsafe {
            jolt_ffi::jolt_body_get_linear_velocity(self.world, body.0, velocity.as_mut_ptr());
        }
        Vec3::from(velocity)
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
