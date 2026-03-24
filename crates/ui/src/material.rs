use ecs::component::Component;
use essential::assets::Asset;
use render::{assets::material::Material, assets::vertex::VertexBufferLayout, AsBindGroup};

use crate::vertex::UIVertex;

/// Material for UI elements.
///
/// Carries a solid `color` (RGBA, each channel in the range `0.0 – 1.0`) that
/// is uploaded as a uniform buffer and bound at `@group(0) @binding(0)` in the
/// UI shader.
///
/// The bind-group layout and bind-group creation are fully macro-generated via
/// `#[derive(AsBindGroup)]`, so the layout definition always stays in sync with
/// the struct definition.
///
/// Register via `MaterialPlugin::<UIMaterial>::pipeline_only()` to create the
/// wgpu pipeline without adding the generic mesh rendering systems.
#[derive(Component, Asset, AsBindGroup)]
#[material(
    vertex_shader = include_str!("shaders/ui.wgsl"),
    fragment_shader = include_str!("shaders/ui.wgsl")
)]
pub struct UIMaterial {
    /// RGBA colour as `[r, g, b, a]` with values in `[0.0, 1.0]`.
    #[uniform(0)]
    pub color: [f32; 4],
}

impl Material for UIMaterial {
    fn needs_camera() -> bool {
        false
    }

    fn needs_lighting() -> bool {
        false
    }

    fn needs_skeleton() -> bool {
        false
    }

    fn vertex_layouts() -> Vec<wgpu::VertexBufferLayout<'static>> {
        vec![UIVertex::describe()]
    }

    fn depth_stencil() -> Option<wgpu::DepthStencilState> {
        None
    }
}
