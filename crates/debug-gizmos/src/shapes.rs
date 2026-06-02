use ecs::Resource;
use essential::assets::handle::AssetHandle;
use render::assets::{mesh::Mesh, vertex::Vertex};

#[derive(Resource)]
pub(crate) struct GizmoShapes {
    pub(crate) line: AssetHandle<Mesh>,
    pub(crate) sphere: AssetHandle<Mesh>,
    pub(crate) cube: AssetHandle<Mesh>,
}

impl GizmoShapes {
    pub fn make_line() -> Mesh {
        let mut start = Vertex::default();
        start.pos_coords = [0.0, 0.0, 0.0];

        let mut end = Vertex::default();
        end.pos_coords = [1.0, 0.0, 0.0];
        Mesh {
            vertices: vec![start, end],
            indices: vec![0, 1],
        }
    }

    pub fn make_unit_sphere() -> Mesh {
        const SEGMENTS: usize = 32;
        let mut vertices: Vec<Vertex> = Vec::with_capacity(SEGMENTS * 3);
        let mut indices: Vec<u32> = Vec::with_capacity(SEGMENTS * 3 * 2);

        for ring in 0..3 {
            let base = (ring * SEGMENTS) as u32;
            for i in 0..SEGMENTS {
                let angle = (i as f32 / SEGMENTS as f32) * std::f32::consts::TAU;
                let (s, c) = angle.sin_cos();
                let pos = match ring {
                    0 => [c, s, 0.0],      // XY plane
                    1 => [c, 0.0, s],      // XZ plane
                    _ => [0.0, c, s],      // YZ plane
                };
                let mut v = Vertex::default();
                v.pos_coords = pos;
                vertices.push(v);

                let next = base + ((i + 1) % SEGMENTS) as u32;
                indices.push(base + i as u32);
                indices.push(next);
            }
        }

        Mesh { vertices, indices }
    }

    pub fn make_unit_cube() -> Mesh {
        #[rustfmt::skip]
        let corners: [[f32; 3]; 8] = [
            [-0.5, -0.5, -0.5], // 0
            [ 0.5, -0.5, -0.5], // 1
            [ 0.5,  0.5, -0.5], // 2
            [-0.5,  0.5, -0.5], // 3
            [-0.5, -0.5,  0.5], // 4
            [ 0.5, -0.5,  0.5], // 5
            [ 0.5,  0.5,  0.5], // 6
            [-0.5,  0.5,  0.5], // 7
        ];

        let vertices = corners.iter().map(|&pos| {
            let mut v = Vertex::default();
            v.pos_coords = pos;
            v
        }).collect();

        #[rustfmt::skip]
        let indices = vec![
            0, 1,  1, 2,  2, 3,  3, 0, // bottom face
            4, 5,  5, 6,  6, 7,  7, 4, // top face
            0, 4,  1, 5,  2, 6,  3, 7, // vertical edges
        ];

        Mesh { vertices, indices }
    }
}
