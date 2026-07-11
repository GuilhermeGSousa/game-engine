#pragma once

#include <stdint.h>

#ifdef __cplusplus
extern "C"
{
#endif

    /* Owns the JPH::PhysicsSystem plus the C++ collision-layer interface
     * implementations it references. */
    typedef struct JoltWorld JoltWorld;

    /* Owns the per-step scratch: a JPH::TempAllocatorImpl and a
     * JPH::JobSystemThreadPool. */
    typedef struct JoltStepper JoltStepper;

    /* Process-global one-time Jolt setup (allocator, factory, type registry).
     * Thread-safe and idempotent; jolt_world_create calls it implicitly. */
    void jolt_global_init(void);

    /* Creates a physics world with Jolt's default gravity (0, -9.81, 0).
     * num_body_mutexes may be 0 to let Jolt pick a default. */
    JoltWorld *jolt_world_create(uint32_t max_bodies,
                                 uint32_t num_body_mutexes,
                                 uint32_t max_body_pairs,
                                 uint32_t max_contact_constraints);
    void jolt_world_destroy(JoltWorld *world);

    /* Creates the step scratch. The thread pool sizes itself to the machine
     * (hardware concurrency - 1 workers). */
    JoltStepper *jolt_stepper_create(uint32_t temp_allocator_bytes);
    void jolt_stepper_destroy(JoltStepper *stepper);

    /* Advances the world by delta_time. Returns JPH::EPhysicsUpdateError bits
     * (0 = no error). */
    uint32_t jolt_world_step(JoltWorld *world,
                             JoltStepper *stepper,
                             float delta_time,
                             int collision_steps);

#ifdef __cplusplus
}
#endif
