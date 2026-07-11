//! Hand-written FFI bindings to the Jolt Physics library.
//!
//! The C API is our own thin shim (headers and sources in `csrc/`) over the
//! unmodified Jolt sources at `vendor/JoltPhysics` — a git submodule pinned
//! to the upstream v5.0.0 release (MIT licensed, see
//! `vendor/JoltPhysics/LICENSE`). Run `git submodule update --init` after
//! cloning to fetch it. Everything — Jolt and the shim — is compiled by
//! `build.rs` in a single `cc` invocation, so there is no external binding
//! crate, no bindgen, and no CMake involved.
//!
//! The declarations below must mirror the `csrc` headers exactly.

use std::marker::{PhantomData, PhantomPinned};

/// Opaque Jolt world: the physics system plus the collision-layer interfaces
/// it references (owned on the C++ side).
#[repr(C)]
pub struct JoltWorld {
    _data: [u8; 0],
    _marker: PhantomData<(*mut u8, PhantomPinned)>,
}

/// Opaque per-step scratch: temp allocator + job system thread pool.
#[repr(C)]
pub struct JoltStepper {
    _data: [u8; 0],
    _marker: PhantomData<(*mut u8, PhantomPinned)>,
}

/// Opaque body recipe for [`jolt_body_create`] (wraps
/// `JPH::BodyCreationSettings`).
#[repr(C)]
pub struct JoltBodyCreationSettings {
    _data: [u8; 0],
    _marker: PhantomData<(*mut u8, PhantomPinned)>,
}

/// A Jolt body id (`JPH::BodyID::GetIndexAndSequenceNumber()`).
pub type JoltBodyId = u32;

/// Mirrors `JPH::EMotionType`. Static bodies live on the NON_MOVING collision
/// layer, kinematic and dynamic bodies on MOVING.
pub type JoltMotionType = u32;
pub const JOLT_MOTION_TYPE_STATIC: JoltMotionType = 0;
pub const JOLT_MOTION_TYPE_KINEMATIC: JoltMotionType = 1;
pub const JOLT_MOTION_TYPE_DYNAMIC: JoltMotionType = 2;

/// Bitmask of the degrees of freedom a dynamic body may use (mirrors
/// `JPH::EAllowedDOFs`). A value of 0 is invalid: use a static body instead.
pub type JoltAllowedDofs = u32;
pub const JOLT_ALLOWED_DOFS_TRANSLATION_X: JoltAllowedDofs = 1 << 0;
pub const JOLT_ALLOWED_DOFS_TRANSLATION_Y: JoltAllowedDofs = 1 << 1;
pub const JOLT_ALLOWED_DOFS_TRANSLATION_Z: JoltAllowedDofs = 1 << 2;
pub const JOLT_ALLOWED_DOFS_ROTATION_X: JoltAllowedDofs = 1 << 3;
pub const JOLT_ALLOWED_DOFS_ROTATION_Y: JoltAllowedDofs = 1 << 4;
pub const JOLT_ALLOWED_DOFS_ROTATION_Z: JoltAllowedDofs = 1 << 5;
pub const JOLT_ALLOWED_DOFS_ALL: JoltAllowedDofs = 0x3f;

/// The closest hit of a raycast (mirrors `JoltRayHit` in `ray.h`).
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct JoltRayHit {
    /// The body that was hit.
    pub body: JoltBodyId,
    /// Hit distance as a fraction `[0, 1]` of the ray's direction vector.
    pub fraction: f32,
    /// World-space surface normal at the hit point (xyz).
    pub normal: [f32; 3],
}

extern "C" {
    /// Process-global one-time Jolt setup. Thread-safe and idempotent;
    /// `jolt_world_create` calls it implicitly.
    pub fn jolt_global_init();

    /// Creates a physics world with Jolt's default gravity (0, -9.81, 0).
    /// `num_body_mutexes` may be 0 to let Jolt pick a default.
    pub fn jolt_world_create(
        max_bodies: u32,
        num_body_mutexes: u32,
        max_body_pairs: u32,
        max_contact_constraints: u32,
    ) -> *mut JoltWorld;

    pub fn jolt_world_destroy(world: *mut JoltWorld);

    /// Creates the step scratch. The thread pool sizes itself to the machine.
    pub fn jolt_stepper_create(temp_allocator_bytes: u32) -> *mut JoltStepper;

    pub fn jolt_stepper_destroy(stepper: *mut JoltStepper);

    /// Advances the world. Returns `JPH::EPhysicsUpdateError` bits (0 = ok).
    pub fn jolt_world_step(
        world: *mut JoltWorld,
        stepper: *mut JoltStepper,
        delta_time: f32,
        collision_steps: i32,
    ) -> u32;

    /// Creates settings at the origin with an identity rotation and static
    /// motion; a shape must be set before the settings are used.
    pub fn jolt_body_creation_settings_create() -> *mut JoltBodyCreationSettings;

    pub fn jolt_body_creation_settings_destroy(settings: *mut JoltBodyCreationSettings);

    pub fn jolt_body_creation_settings_set_position(
        settings: *mut JoltBodyCreationSettings,
        position: *const f32,
    );

    pub fn jolt_body_creation_settings_set_rotation(
        settings: *mut JoltBodyCreationSettings,
        rotation: *const f32,
    );

    pub fn jolt_body_creation_settings_set_motion_type(
        settings: *mut JoltBodyCreationSettings,
        motion_type: JoltMotionType,
    );

    /// Only meaningful for dynamic bodies (e.g. lock the rotation DOFs to
    /// keep a player capsule upright). Defaults to all.
    pub fn jolt_body_creation_settings_set_allowed_dofs(
        settings: *mut JoltBodyCreationSettings,
        allowed_dofs: JoltAllowedDofs,
    );

    /// `density` (kg/m³) sets the shape's density, from which Jolt derives a
    /// dynamic body's mass; it has no effect on static bodies.
    pub fn jolt_body_creation_settings_set_sphere_shape(
        settings: *mut JoltBodyCreationSettings,
        radius: f32,
        density: f32,
    );

    pub fn jolt_body_creation_settings_set_box_shape(
        settings: *mut JoltBodyCreationSettings,
        half_extents: *const f32,
        density: f32,
    );

    /// A capsule with total height `2 * (half_height + radius)`: a cylinder
    /// of `2 * half_height` capped by hemispheres of `radius`, along the
    /// local Y axis.
    pub fn jolt_body_creation_settings_set_capsule_shape(
        settings: *mut JoltBodyCreationSettings,
        half_height: f32,
        radius: f32,
        density: f32,
    );

    /// Creates a body from `settings` and adds it to the simulation, active
    /// unless static. The settings remain owned by the caller and can be
    /// reused.
    pub fn jolt_body_create(
        world: *mut JoltWorld,
        settings: *const JoltBodyCreationSettings,
    ) -> JoltBodyId;

    /// Removes a body from the simulation and destroys it, waking any bodies
    /// that were touching it (they would otherwise sleep in mid-air). The id
    /// is invalid afterwards.
    pub fn jolt_body_destroy(world: *mut JoltWorld, body: JoltBodyId);

    /// Reads a body's world-space position (xyz) and rotation (xyzw).
    pub fn jolt_body_get_transform(
        world: *const JoltWorld,
        body: JoltBodyId,
        out_position: *mut f32,
        out_rotation: *mut f32,
    );

    /// Casts a ray from `origin` (xyz) along `direction` (xyz, whose length is
    /// the maximum cast distance) against all bodies, writing the closest hit
    /// to `out_hit`. Returns `false` — leaving `out_hit` untouched — if
    /// nothing was hit.
    pub fn jolt_world_cast_ray(
        world: *const JoltWorld,
        origin: *const f32,
        direction: *const f32,
        out_hit: *mut JoltRayHit,
    ) -> bool;
}
