use essential::assets::{handle::AssetHandle, Asset};

use crate::{
    assets::{material::Material, texture::Texture},
    AsBindGroup,
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
/// * This material is intended for use with the engine's built-in skybox
///   render pass, not through [`crate::material_plugin::MaterialPlugin`].
#[derive(Asset, AsBindGroup)]
#[material(
    vertex_shader = include_str!("../shaders/skybox.wgsl"),
    fragment_shader = include_str!("../shaders/skybox.wgsl")
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
}
