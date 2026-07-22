#pragma once

#include <stdint.h>

/* Collision shapes, created independently of the bodies that use them.
 *
 * Shapes are immutable and refcounted: one shape may back any number of
 * bodies, so a mesh's triangle BVH can be built once and shared by every
 * entity using that mesh. A JoltShape handle owns one reference and bodies
 * take their own, so destroying the handle while bodies still use the shape
 * is safe.
 *
 * Half-extents are xyz float triples. Plain float arrays are used instead of
 * structs so the ABI is independent of Jolt's 16-byte-aligned vector types. */

#ifdef __cplusplus
extern "C"
{
#endif

    typedef struct JoltShape JoltShape;

    /* `density` (kg/m^3) is the density of the shape's volume, from which
     * Jolt derives a dynamic body's mass; it has no effect on static bodies. */
    JoltShape *jolt_create_sphere_shape(float radius, float density);
    JoltShape *jolt_create_box_shape(const float half_extents[3], float density);

    /* A capsule with total height 2 * (half_height + radius): a cylinder of
     * 2 * half_height capped by hemispheres of `radius`, along the local Y
     * axis. */
    JoltShape *jolt_create_capsule_shape(float half_height, float radius, float density);

    /* A triangle mesh. `vertices` is `vertex_count` xyz triples; `indices` is
     * `index_count` vertex indices, three per triangle, wound counter-clockwise.
     * Triangles are single sided for simulation. Returns NULL if Jolt rejected
     * the mesh.
     *
     * Takes no density: a mesh need not form a closed hull, so it has no
     * volume to derive a mass from, and Jolt requires mesh-shaped bodies to be
     * static (JPH::MeshShape::MustBeStatic). */
    JoltShape *jolt_create_mesh_shape(const float *vertices,
                                      uint32_t vertex_count,
                                      const uint32_t *indices,
                                      uint32_t index_count);

    /* Releases the handle's reference. Bodies already created from the shape
     * keep theirs and stay valid. */
    void jolt_shape_destroy(JoltShape *shape);

    /* The shape's axis-aligned bounds in its own local space, before any body
     * position, rotation or scale. Used to derive dimensions the engine no
     * longer tracks itself (e.g. how far a shape extends below its origin). */
    void jolt_shape_get_local_bounds(const JoltShape *shape,
                                     float out_min[3],
                                     float out_max[3]);

#ifdef __cplusplus
}
#endif
