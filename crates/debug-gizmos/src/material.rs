use essential::assets::Asset;
use render::AsBindGroup;



#[derive(Asset, AsBindGroup)]
#[material(
    vertex_shader = include_str!("shaders/gizmo.wgsl"),
    fragment_shader = include_str!("shaders/gizmo.wgsl")
)]
pub(crate) struct DebugGizmoMaterial
{

}