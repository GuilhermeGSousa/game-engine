use essential::assets::{Asset, handle::AssetHandle};

use render::{
    AsBindGroup, Material,
    assets::{
        texture::Texture,
        vertex::{Vertex, VertexBufferLayout},
    },
};

/// Material used by the skybox render pass.
///
/// Wraps an optional cube-map texture handle and implements [`AsBindGroup`]
/// via the derive macro, making the bind-group layout and bind-group creation
/// fully macro-generated.
///
/// The material is placed at `@group(0)` in the skybox shader, and the
/// standard camera uniform lives at `@group(1)`.
///
/// # Differences from regular mesh materials
///
/// * `cull_mode` is [`wgpu::Face::Front`] instead of the default
///   `wgpu::Face::Back` – necessary for rendering the inside of a cube.
/// * `depth_stencil` returns `None` – the skybox does not interact with the
///   depth buffer.
/// * `vertex_layouts` returns only [`SkyboxVertex`] – the skybox cube uses a
///   single position-only vertex buffer with no per-instance transform.
/// * This material is intended to be registered via
///   [`crate::material_plugin::MaterialPlugin::pipeline_only`] which creates
///   the [`crate::material_plugin::MaterialPipeline`] resource without adding
///   the generic mesh rendering systems.
#[derive(Asset, AsBindGroup)]
#[material(
    vertex_shader = include_str!("shaders/skybox.wgsl"),
    fragment_shader = include_str!("shaders/skybox.wgsl")
)]
pub struct SkyboxMaterial {
    /// The cube-map texture (binding 0) and its sampler (binding 1).
    #[texture(0, dimension = "cube")]
    #[sampler(1)]
    pub texture: Option<AssetHandle<Texture>>,
}

impl Material for SkyboxMaterial {
    fn needs_camera() -> bool {
        true
    }

    fn needs_lighting() -> bool {
        false
    }

    fn needs_skeleton() -> bool {
        false
    }

    fn cull_mode() -> Option<wgpu::Face> {
        Some(wgpu::Face::Front)
    }

    fn vertex_layouts() -> Vec<wgpu::VertexBufferLayout<'static>> {
        vec![Vertex::describe()]
    }

    fn depth_stencil() -> Option<wgpu::DepthStencilState> {
        Some(wgpu::DepthStencilState {
            format: wgpu::TextureFormat::Depth32Float,
            depth_write_enabled: false,
            depth_compare: wgpu::CompareFunction::LessEqual,
            stencil: wgpu::StencilState::default(),
            bias: wgpu::DepthBiasState::default(),
        })
    }
}
