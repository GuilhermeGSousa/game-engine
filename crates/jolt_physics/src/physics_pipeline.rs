use ecs::resource::Resource;
use essential::time::Time;

use crate::physics_state::PhysicsState;

/// Per-step scratch resources for advancing the simulation: a temporary
/// allocator and a job system thread pool (owned together on the C++ side of
/// the `jolt-sys` shim). Owns them for the lifetime of the app.
#[derive(Resource)]
pub struct PhysicsPipeline {
    stepper: *mut jolt_sys::JoltStepper,
}

// SAFETY: like `PhysicsState`, this holds a raw Jolt pointer. The ECS requires
// `Resource: Send + Sync`; the scheduler guarantees exclusive `ResMut` access,
// so the pointer is never used from two threads concurrently.
unsafe impl Send for PhysicsPipeline {}
unsafe impl Sync for PhysicsPipeline {}

impl PhysicsPipeline {
    pub fn new() -> Self {
        // SAFETY: `jolt_stepper_create` returns an owned stepper which we free
        // in `Drop`.
        //
        // 32 MiB of scratch, sized to comfortably hold the body-pair and
        // contact-constraint buffers Jolt allocates each step for
        // `PhysicsState`'s configured maxima.
        let stepper = unsafe { jolt_sys::jolt_stepper_create(32 * 1024 * 1024) };

        PhysicsPipeline { stepper }
    }

    /// Advances `state` by one fixed timestep.
    pub fn step(&mut self, state: &mut PhysicsState) {
        // SAFETY: `state.world()` is valid, and the stepper is owned by
        // `self`. We hold `&mut self` and `&mut state`, so this is the
        // exclusive accessor.
        unsafe {
            jolt_sys::jolt_world_step(state.world(), self.stepper, Time::fixed_delta_time(), 1);
        }
    }
}

impl Default for PhysicsPipeline {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for PhysicsPipeline {
    fn drop(&mut self) {
        // SAFETY: the stepper was created in `new` and is freed once here.
        unsafe {
            jolt_sys::jolt_stepper_destroy(self.stepper);
        }
    }
}
