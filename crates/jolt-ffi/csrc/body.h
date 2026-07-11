#pragma once

#include <stdint.h>

#include "world.h"

/* Positions and half-extents are xyz float triples; rotations are xyzw
 * quaternions. Plain float arrays are used instead of structs so the ABI is
 * independent of Jolt's 16-byte-aligned vector types. */

#ifdef __cplusplus
extern "C"
{
#endif

    /* A Jolt body id (JPH::BodyID::GetIndexAndSequenceNumber()). */
    typedef uint32_t JoltBodyId;

    /* Creates an active dynamic body on the MOVING layer at `position` with an
     * identity rotation and a sphere shape of `radius`. `density` (kg/m^3) sets
     * the shape's density, from which Jolt derives the body's mass. */
    JoltBodyId jolt_body_create_dynamic_sphere(JoltWorld *world,
                                               const float position[3],
                                               float radius,
                                               float density);

    /* Like jolt_body_create_dynamic_sphere, with a box shape of the given
     * half-extents. */
    JoltBodyId jolt_body_create_dynamic_box(JoltWorld *world,
                                            const float position[3],
                                            const float half_extents[3],
                                            float density);

    /* Creates an inactive static body on the NON_MOVING layer at `position` with
     * a box shape of the given half-extents. */
    JoltBodyId jolt_body_create_static_box(JoltWorld *world,
                                           const float position[3],
                                           const float half_extents[3]);

    /* Like jolt_body_create_static_box, with a sphere shape of `radius`. */
    JoltBodyId jolt_body_create_static_sphere(JoltWorld *world,
                                              const float position[3],
                                              float radius);

    /* Removes a body from the simulation and destroys it, waking any bodies that
     * were touching it (they would otherwise sleep in mid-air). The id is invalid
     * afterwards. */
    void jolt_body_destroy(JoltWorld *world, JoltBodyId body);

    /* Reads a body's world-space position (xyz) and rotation (xyzw). */
    void jolt_body_get_transform(const JoltWorld *world,
                                 JoltBodyId body,
                                 float out_position[3],
                                 float out_rotation[4]);

#ifdef __cplusplus
}
#endif
