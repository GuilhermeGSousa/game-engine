use ecs::resource::Resource;
use essential::transform::Transform;
use joltc_sys::*;

use crate::body::BodyId;
use crate::collider::Collider;
use crate::ffi_util::{create_box, create_sphere, quat_to_glam, rvec3, rvec3_to_glam, vec3};
use crate::init::global_init;
use crate::layers::{LayerInterfaces, OL_NON_MOVING};
use crate::rigid_body::RigidBody;

const MAX_BODIES: u32 = 10_240;
const NUM_BODY_MUTEXES: u32 = 0; // 0 = let Jolt pick a default
// These bound the per-step scratch Jolt allocates from the temp allocator
// (see `PhysicsPipeline`). Keep them in step with that allocator's size.
const MAX_BODY_PAIRS: u32 = 10_240;
const MAX_CONTACT_CONSTRAINTS: u32 = 10_240;

/// Owns the Jolt physics world: the body store, broad/narrow phases, and the
/// collision-layer interfaces it was initialised with.
#[derive(Resource)]
pub struct PhysicsState {
    /// The Jolt physics system. Declared first so it is dropped before
    /// `layers` — `Init` stores raw pointers into the layer interfaces, so they
    /// must outlive the system. (Drop is explicit below, but field order keeps
    /// the invariant obvious.)
    system: *mut JPC_PhysicsSystem,
    /// Kept alive so the interfaces `system` references outlive it; only used
    /// via its `Drop`, hence `dead_code` is expected.
    #[allow(dead_code)]
    layers: LayerInterfaces,
}

// SAFETY: `PhysicsState` holds raw pointers into Jolt, which are not inherently
// `Send`/`Sync`. The ECS requires `Resource: Send + Sync`, and the scheduler
// upholds the actual safety contract: a `ResMut<PhysicsState>` is granted
// exclusive access, so the system is never touched from two threads at once.
unsafe impl Send for PhysicsState {}
unsafe impl Sync for PhysicsState {}

impl PhysicsState {
    pub fn new() -> Self {
        global_init();

        let layers = LayerInterfaces::new();

        // SAFETY: `JPC_PhysicsSystem_new` returns an owned system, and the
        // layer interfaces live in `layers` for as long as `self` does.
        let system = unsafe {
            let system = JPC_PhysicsSystem_new();
            JPC_PhysicsSystem_Init(
                system,
                MAX_BODIES,
                NUM_BODY_MUTEXES,
                MAX_BODY_PAIRS,
                MAX_CONTACT_CONSTRAINTS,
                layers.broad_phase_layer_interface,
                layers.object_vs_broad_phase_layer_filter,
                layers.object_layer_pair_filter,
            );
            system
        };

        Self { system, layers }
    }

    /// The raw Jolt system pointer, used by [`PhysicsPipeline`] to drive
    /// stepping.
    ///
    /// [`PhysicsPipeline`]: crate::physics_pipeline::PhysicsPipeline
    pub(crate) fn system(&self) -> *mut JPC_PhysicsSystem {
        self.system
    }

    /// The body interface for this system. The returned pointer borrows from
    /// `self` and must not outlive it.
    pub(crate) fn body_interface(&self) -> *mut JPC_BodyInterface {
        // SAFETY: `self.system` is a valid, initialised system.
        unsafe { JPC_PhysicsSystem_GetBodyInterface(self.system) }
    }

    /// Replaces a body's shape with a sphere of the given radius.
    pub fn make_sphere(&mut self, parent: &RigidBody, radius: f32) -> Collider {
        let shape = create_sphere(radius);
        let body = **parent; // BodyId (Copy) via Deref

        // SAFETY: `body` refers to a body that was added to this system, and
        // `shape` is a freshly created valid shape.
        unsafe {
            JPC_BodyInterface_SetShape(
                self.body_interface(),
                body.0,
                shape,
                true,
                JPC_ACTIVATION_ACTIVATE,
            );
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
        let shape = create_box(vec3(width, height, length));

        match parent {
            Some(rb) => {
                let body = **rb; // BodyId (Copy) via Deref

                // SAFETY: `body` is a body in this system; `shape` is valid.
                unsafe {
                    JPC_BodyInterface_SetShape(
                        self.body_interface(),
                        body.0,
                        shape,
                        true,
                        JPC_ACTIVATION_ACTIVATE,
                    );
                }
                Collider(body)
            }
            None => {
                let settings = JPC_BodyCreationSettings {
                    Position: rvec3(transform.translation),
                    MotionType: JPC_MOTION_TYPE_STATIC,
                    ObjectLayer: OL_NON_MOVING,
                    Shape: shape,
                    ..Default::default()
                };

                // SAFETY: `settings` is fully initialised with a valid shape.
                let id = unsafe {
                    JPC_BodyInterface_CreateAndAddBody(
                        self.body_interface(),
                        &settings,
                        JPC_ACTIVATION_DONT_ACTIVATE,
                    )
                };

                Collider(BodyId(id))
            }
        }
    }

    /// Reads a body's current world transform out of the simulation.
    pub fn get_rigid_body(&self, rigid_body: &RigidBody) -> Transform {
        let bi = self.body_interface();
        let body = **rigid_body; // BodyId (Copy) via Deref

        // SAFETY: `body` is a valid body id within this system.
        let (position, rotation) = unsafe {
            (
                JPC_BodyInterface_GetPosition(bi, body.0),
                JPC_BodyInterface_GetRotation(bi, body.0),
            )
        };

        Transform::from_translation_rotation(rvec3_to_glam(position), quat_to_glam(rotation))
    }
}

impl Default for PhysicsState {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for PhysicsState {
    fn drop(&mut self) {
        // SAFETY: `self.system` was created in `new` and is deleted exactly
        // once here. It is freed before `self.layers` (which Rust drops
        // afterwards), satisfying the requirement that the layer interfaces
        // outlive the system.
        unsafe {
            JPC_PhysicsSystem_delete(self.system);
        }
    }
}
