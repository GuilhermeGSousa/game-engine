#[repr(C)]
#[derive(Copy, Clone, Debug, Default)]
pub struct Vertex {
    pub pos_coords: [f32; 3],
    pub uv_coords: [f32; 2],
    pub normal: [f32; 3],
    pub tangent: [f32; 3],
    pub bitangent: [f32; 3],
    pub bone_indices: [u32; Vertex::MAX_AFFECTED_BONES],
    pub bone_weights: [f32; Vertex::MAX_AFFECTED_BONES],
}

impl Vertex {
    pub const MAX_AFFECTED_BONES: usize = 4;
}

unsafe impl bytemuck::Pod for Vertex {}
unsafe impl bytemuck::Zeroable for Vertex {}
