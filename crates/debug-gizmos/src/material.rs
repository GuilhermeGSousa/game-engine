use color::LinearRgba;
use essential::assets::Asset;
use render::{AsBindGroup, Material};

#[derive(Asset, AsBindGroup)]
#[material(
    vertex_shader = include_str!("shaders/gizmo.wgsl"),
    fragment_shader = include_str!("shaders/gizmo.wgsl")
)]
pub(crate) struct DebugGizmoMaterial {
    #[uniform(0)]
    pub color: LinearRgba,
}

impl Material for DebugGizmoMaterial {
    fn depth_stencil() -> Option<wgpu::DepthStencilState>
    where
        Self: Sized,
    {
        None
    }

    fn topology() -> wgpu::PrimitiveTopology
    where
        Self: Sized,
    {
        wgpu::PrimitiveTopology::LineList
    }

    fn clear_depth() -> bool
    where
        Self: Sized,
    {
        false
    }
}
