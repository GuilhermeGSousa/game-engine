use glam::Vec3;
use mesh::{Mesh, Vertex};

/// Builds a solid, axis-aligned box mesh centered on the origin with the given half-extents.
///
/// Each face gets its own 4 vertices with a flat per-face normal (not smoothed), which is the
/// correct look for blocky level geometry. Winding is counter-clockwise as seen from outside
/// each face, matching the renderer's `FrontFace::Ccw` + back-face-culling convention.
pub fn make_box_mesh(half_extents: Vec3) -> Mesh {
    make_box_mesh_offset(Vec3::ZERO, half_extents)
}

/// Like [`make_box_mesh`], but every vertex is additionally shifted by `center`, so the mesh's
/// pivot (origin, where its owning entity's [`essential::transform::Transform`] sits) doesn't
/// have to be the box's geometric center. Useful for limb-style meshes that hang or extend away
/// from a joint's pivot rather than being centered on it.
pub fn make_box_mesh_offset(center: Vec3, half_extents: Vec3) -> Mesh {
    let (hx, hy, hz) = (half_extents.x, half_extents.y, half_extents.z);

    // Each entry: the 4 corners of one face, in CCW order as seen from outside, and the
    // face's outward normal.
    #[rustfmt::skip]
    let faces: [([[f32; 3]; 4], [f32; 3]); 6] = [
        // +X
        ([[hx, -hy, -hz], [hx, hy, -hz], [hx, hy, hz], [hx, -hy, hz]], [1.0, 0.0, 0.0]),
        // -X
        ([[-hx, -hy, -hz], [-hx, -hy, hz], [-hx, hy, hz], [-hx, hy, -hz]], [-1.0, 0.0, 0.0]),
        // +Y (top)
        ([[-hx, hy, -hz], [-hx, hy, hz], [hx, hy, hz], [hx, hy, -hz]], [0.0, 1.0, 0.0]),
        // -Y (bottom)
        ([[-hx, -hy, -hz], [hx, -hy, -hz], [hx, -hy, hz], [-hx, -hy, hz]], [0.0, -1.0, 0.0]),
        // +Z
        ([[-hx, -hy, hz], [hx, -hy, hz], [hx, hy, hz], [-hx, hy, hz]], [0.0, 0.0, 1.0]),
        // -Z
        ([[-hx, -hy, -hz], [-hx, hy, -hz], [hx, hy, -hz], [hx, -hy, -hz]], [0.0, 0.0, -1.0]),
    ];

    let mut vertices = Vec::with_capacity(24);
    let mut indices = Vec::with_capacity(36);

    #[rustfmt::skip]
    let uvs: [[f32; 2]; 4] = [[0.0, 0.0], [1.0, 0.0], [1.0, 1.0], [0.0, 1.0]];

    for (corners, normal) in faces {
        let base = vertices.len() as u32;
        for (corner, uv) in corners.iter().zip(uvs.iter()) {
            let pos = Vec3::from_array(*corner) + center;
            vertices.push(Vertex {
                pos_coords: pos.to_array(),
                uv_coords: *uv,
                normal,
                ..Vertex::default()
            });
        }
        indices.extend_from_slice(&[base, base + 1, base + 2, base, base + 2, base + 3]);
    }

    let mut mesh = Mesh { vertices, indices };
    mesh.compute_tangents();
    mesh
}
