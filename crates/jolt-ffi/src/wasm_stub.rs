//! Inert stand-ins for the Jolt shim on `wasm32` targets.
//!
//! Jolt's core requires `std::mutex`/`std::thread`, which the C++ toolchains
//! for `wasm32-unknown-unknown` (wasi-libc without pthread stubs, libc++ built
//! with `_LIBCPP_HAS_NO_THREADS`) cannot provide — Jolt's own web builds go
//! through emscripten. Until the engine has an emscripten-based web physics
//! build, these stubs keep wasm targets compiling and linking with degraded
//! behavior instead of failing the build:
//!
//! - bodies remember the pose they were created with and never move,
//! - stepping is a no-op,
//! - raycasts never hit anything.
//!
//! The signatures mirror the `extern "C"` declarations in `lib.rs` exactly
//! (including `unsafe`), so `jolt_physics` compiles unchanged. The safety
//! contracts are the ones documented there: valid world/stepper pointers from
//! the matching `create` functions and valid xyz/xyzw buffers.
#![allow(clippy::missing_safety_doc)]

use std::collections::HashMap;

use crate::{JoltBodyId, JoltRayHit, JoltStepper, JoltWorld};

/// Per-world stub state: the pose each body was created with.
struct StubWorld {
    bodies: HashMap<JoltBodyId, [f32; 3]>,
    next_id: JoltBodyId,
}

/// # Safety
/// `world` must be a pointer previously returned by [`jolt_world_create`].
unsafe fn stub(world: *mut JoltWorld) -> &'static mut StubWorld {
    &mut *(world as *mut StubWorld)
}

pub unsafe fn jolt_global_init() {}

pub unsafe fn jolt_world_create(
    _max_bodies: u32,
    _num_body_mutexes: u32,
    _max_body_pairs: u32,
    _max_contact_constraints: u32,
) -> *mut JoltWorld {
    Box::into_raw(Box::new(StubWorld {
        bodies: HashMap::new(),
        next_id: 0,
    })) as *mut JoltWorld
}

pub unsafe fn jolt_world_destroy(world: *mut JoltWorld) {
    drop(Box::from_raw(world as *mut StubWorld));
}

pub unsafe fn jolt_stepper_create(_temp_allocator_bytes: u32) -> *mut JoltStepper {
    Box::into_raw(Box::new(0u8)) as *mut JoltStepper
}

pub unsafe fn jolt_stepper_destroy(stepper: *mut JoltStepper) {
    drop(Box::from_raw(stepper as *mut u8));
}

pub unsafe fn jolt_world_step(
    _world: *mut JoltWorld,
    _stepper: *mut JoltStepper,
    _delta_time: f32,
    _collision_steps: i32,
) -> u32 {
    0
}

unsafe fn create_body(world: *mut JoltWorld, position: *const f32) -> JoltBodyId {
    let world = stub(world);
    let id = world.next_id;
    world.next_id += 1;
    world
        .bodies
        .insert(id, [*position, *position.add(1), *position.add(2)]);
    id
}

pub unsafe fn jolt_body_create_dynamic(
    world: *mut JoltWorld,
    position: *const f32,
    _placeholder_radius: f32,
) -> JoltBodyId {
    create_body(world, position)
}

pub unsafe fn jolt_body_create_static_box(
    world: *mut JoltWorld,
    position: *const f32,
    _half_extents: *const f32,
) -> JoltBodyId {
    create_body(world, position)
}

pub unsafe fn jolt_body_set_sphere_shape(
    _world: *mut JoltWorld,
    _body: JoltBodyId,
    _radius: f32,
) {
}

pub unsafe fn jolt_body_set_box_shape(
    _world: *mut JoltWorld,
    _body: JoltBodyId,
    _half_extents: *const f32,
) {
}

pub unsafe fn jolt_body_get_transform(
    world: *const JoltWorld,
    body: JoltBodyId,
    out_position: *mut f32,
    out_rotation: *mut f32,
) {
    let position = stub(world as *mut JoltWorld)
        .bodies
        .get(&body)
        .copied()
        .unwrap_or([0.0; 3]);
    for (i, value) in position.into_iter().enumerate() {
        *out_position.add(i) = value;
    }
    // Identity rotation (xyzw).
    for i in 0..3 {
        *out_rotation.add(i) = 0.0;
    }
    *out_rotation.add(3) = 1.0;
}

pub unsafe fn jolt_world_cast_ray(
    _world: *const JoltWorld,
    _origin: *const f32,
    _direction: *const f32,
    _out_hit: *mut JoltRayHit,
) -> bool {
    false
}
