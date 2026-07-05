#pragma once

#include <stdbool.h>
#include <stdint.h>

/* Minimal C API over the vendored Jolt Physics library, covering exactly the
 * feature set the engine uses: a physics world with two collision layers
 * (NON_MOVING statics, MOVING dynamics), dynamic bodies with sphere shapes,
 * static boxes, stepping, and pose read-back.
 *
 * Positions and half-extents are xyz float triples; rotations are xyzw
 * quaternions. Plain float arrays are used instead of structs so the ABI is
 * independent of Jolt's 16-byte-aligned vector types. */

#ifdef __cplusplus
extern "C" {
#endif

/* Owns the JPH::PhysicsSystem plus the C++ collision-layer interface
 * implementations it references. */
typedef struct JoltWorld JoltWorld;

/* Owns the per-step scratch: a JPH::TempAllocatorImpl and a
 * JPH::JobSystemThreadPool. */
typedef struct JoltStepper JoltStepper;

/* A Jolt body id (JPH::BodyID::GetIndexAndSequenceNumber()). */
typedef uint32_t JoltBodyId;

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

/* Creates an active dynamic body on the MOVING layer at `position` with an
 * identity rotation and a placeholder sphere shape of `placeholder_radius`
 * (Jolt requires a shape at creation; replace it with a set-shape call). */
JoltBodyId jolt_body_create_dynamic(JoltWorld *world,
                                    const float position[3],
                                    float placeholder_radius);

/* Creates an inactive static body on the NON_MOVING layer at `position` with
 * a box shape of the given half-extents. */
JoltBodyId jolt_body_create_static_box(JoltWorld *world,
                                       const float position[3],
                                       const float half_extents[3]);

/* Replace a body's shape (mass properties are recomputed and the body is
 * activated). */
void jolt_body_set_sphere_shape(JoltWorld *world, JoltBodyId body, float radius);
void jolt_body_set_box_shape(JoltWorld *world,
                             JoltBodyId body,
                             const float half_extents[3]);

/* Reads a body's world-space position (xyz) and rotation (xyzw). */
void jolt_body_get_transform(const JoltWorld *world,
                             JoltBodyId body,
                             float out_position[3],
                             float out_rotation[4]);

/* The closest hit of a raycast. */
typedef struct JoltRayHit {
	/* The body that was hit. */
	JoltBodyId body;
	/* Hit distance as a fraction [0, 1] of the ray's direction vector. */
	float fraction;
	/* World-space surface normal at the hit point (xyz). */
	float normal[3];
} JoltRayHit;

/* Casts a ray from `origin` along `direction` (whose length is the maximum
 * cast distance) against all bodies, writing the closest hit to `out_hit`.
 * Returns false — leaving `out_hit` untouched — if nothing was hit. */
bool jolt_world_cast_ray(const JoltWorld *world,
                         const float origin[3],
                         const float direction[3],
                         JoltRayHit *out_hit);

#ifdef __cplusplus
}
#endif
