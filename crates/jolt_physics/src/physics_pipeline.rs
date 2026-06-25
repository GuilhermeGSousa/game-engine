use ecs::resource::Resource;
use essential::time::Time;
use joltc_sys::*;

use crate::physics_state::PhysicsState;

/// Per-step scratch resources for advancing the simulation: a temporary
/// allocator and a job system thread pool. Owns both for the lifetime of the
/// app.
#[derive(Resource)]
pub struct PhysicsPipeline {
    temp_allocator: *mut JPC_TempAllocatorImpl,
    job_system: *mut JPC_JobSystemThreadPool,
}

// SAFETY: like `PhysicsState`, this holds raw Jolt pointers. The ECS requires
// `Resource: Send + Sync`; the scheduler guarantees exclusive `ResMut` access,
// so these pointers are never used from two threads concurrently.
unsafe impl Send for PhysicsPipeline {}
unsafe impl Sync for PhysicsPipeline {}

impl PhysicsPipeline {
    pub fn new() -> Self {
        // SAFETY: both constructors return owned Jolt objects which we free in
        // `Drop`.
        unsafe {
            PhysicsPipeline {
                // 32 MiB of scratch, sized to comfortably hold the body-pair
                // and contact-constraint buffers Jolt allocates each step for
                // `PhysicsState`'s configured maxima.
                temp_allocator: JPC_TempAllocatorImpl_new(32 * 1024 * 1024),
                job_system: JPC_JobSystemThreadPool_new2(
                    JPC_MAX_PHYSICS_JOBS as _,
                    JPC_MAX_PHYSICS_BARRIERS as _,
                ),
            }
        }
    }

    /// Advances `state` by one fixed timestep.
    pub fn step(&mut self, state: &mut PhysicsState) {
        // SAFETY: `state.system()` is valid, and the allocator/job-system
        // pointers are owned by `self`. We hold `&mut self` and `&mut state`,
        // so this is the exclusive accessor.
        unsafe {
            JPC_PhysicsSystem_Update(
                state.system(),
                Time::fixed_delta_time(),
                1,
                self.temp_allocator,
                self.job_system,
            );
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
        // SAFETY: both pointers were created in `new` and are freed once here.
        unsafe {
            JPC_JobSystemThreadPool_delete(self.job_system);
            JPC_TempAllocatorImpl_delete(self.temp_allocator);
        }
    }
}
