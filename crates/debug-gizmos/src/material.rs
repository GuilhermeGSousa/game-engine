use essential::assets::Asset;
use render::{AsBindGroup, Material};



#[derive(Asset, AsBindGroup)]
#[material(
    vertex_shader = include_str!("shaders/gizmo.wgsl"),
    fragment_shader = include_str!("shaders/gizmo.wgsl")
)]
pub(crate) struct DebugGizmoMaterial
{

}

impl Material for DebugGizmoMaterial {}
