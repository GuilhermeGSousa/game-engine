use color::LinearRgba;
use essential::assets::Asset;
use render::AsBindGroup;

#[derive(Asset, AsBindGroup)]
#[material(
    vertex_shader = include_str!("shaders/gizmo.wgsl"),
    fragment_shader = include_str!("shaders/gizmo.wgsl"),
    depth_stencil = "none",
    topology = "line_list",
    clear_depth = false,
)]
pub(crate) struct DebugGizmoMaterial {
    #[uniform(0)]
    pub color: LinearRgba,
}
