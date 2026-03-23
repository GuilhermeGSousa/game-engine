/// A simple user-defined material that renders geometry with a flat tint colour.
///
/// This module demonstrates:
/// * Defining a custom material via `#[derive(AsBindGroup)]`.
/// * Pointing to a custom WGSL shader with the `#[material]` attribute using
///   `include_str!` (the shader is resolved relative to this source file).
/// * Registering the material with the engine through [`MaterialPlugin`].
use bytemuck::{Pod, Zeroable};
use essential::assets::Asset;
use render::AsBindGroup;

/// GPU-side uniform for [`UnlitMaterial`].
///
/// `repr(C)` with explicit padding to satisfy the 16-byte alignment required by
/// WebGPU uniform buffers.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct TintUniform {
    /// RGBA tint colour applied to the whole mesh.
    pub color: [f32; 4],
    /// Explicit padding — keeps the struct at 32 bytes (two vec4s).
    pub _padding: [f32; 4],
}

impl TintUniform {
    pub fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self {
            color: [r, g, b, a],
            _padding: [0.0; 4],
        }
    }
}

/// Custom unlit material — renders every fragment with a solid `tint` colour.
///
/// Attach it to a mesh entity via [`CustomMaterialComponent<UnlitMaterial>`]:
///
/// ```rust,ignore
/// let mat_handle = asset_server.add(UnlitMaterial {
///     tint: TintUniform::new(0.2, 0.8, 1.0, 1.0),
/// });
///
/// cmd.spawn((
///     MeshComponent { handle: mesh_handle },
///     CustomMaterialComponent { handle: mat_handle },
///     Transform::default(),
/// ));
/// ```
#[derive(Asset, AsBindGroup)]
#[material(
    vertex_shader = include_str!("shaders/unlit.wgsl"),
    fragment_shader = include_str!("shaders/unlit.wgsl")
)]
pub struct UnlitMaterial {
    /// Tint colour uniform uploaded to the GPU at binding 0.
    #[uniform(0)]
    pub tint: TintUniform,
}
