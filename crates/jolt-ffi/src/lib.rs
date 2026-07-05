//! Hand-written FFI bindings to the Jolt Physics library.
//!
//! The C API is our own thin shim (`csrc/shim.h` / `csrc/shim.cpp`) over the
//! unmodified Jolt sources at `vendor/JoltPhysics` — a git submodule pinned
//! to the upstream v5.0.0 release (MIT licensed, see
//! `vendor/JoltPhysics/LICENSE`). Run `git submodule update --init` after
//! cloning to fetch it. Everything — Jolt and the shim — is compiled by
//! `build.rs` in a single `cc` invocation, so there is no external binding
//! crate, no bindgen, and no CMake involved.
//!
//! The declarations below must mirror `csrc/shim.h` exactly.

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

/// A Jolt body id (`JPH::BodyID::GetIndexAndSequenceNumber()`).
pub type JoltBodyId = u32;

/// The closest hit of a raycast (mirrors `JoltRayHit` in `shim.h`).
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

    /// Creates an active dynamic body on the MOVING layer at `position`
    /// (xyz) with a placeholder sphere shape of `placeholder_radius`.
    pub fn jolt_body_create_dynamic(
        world: *mut JoltWorld,
        position: *const f32,
        placeholder_radius: f32,
    ) -> JoltBodyId;

    /// Creates an inactive static body on the NON_MOVING layer at `position`
    /// (xyz) with a box shape of the given half-extents (xyz).
    pub fn jolt_body_create_static_box(
        world: *mut JoltWorld,
        position: *const f32,
        half_extents: *const f32,
    ) -> JoltBodyId;

    /// Replaces a body's shape with a sphere; mass is recomputed and the body
    /// activated.
    pub fn jolt_body_set_sphere_shape(world: *mut JoltWorld, body: JoltBodyId, radius: f32);

    /// Replaces a body's shape with a box of the given half-extents (xyz);
    /// mass is recomputed and the body activated.
    pub fn jolt_body_set_box_shape(
        world: *mut JoltWorld,
        body: JoltBodyId,
        half_extents: *const f32,
    );

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
