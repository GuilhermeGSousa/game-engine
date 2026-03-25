/// A simple user-defined material that renders geometry with a flat tint colour.
///
/// This module demonstrates the full workflow for defining a custom engine material:
///
/// 1. Derive [`AsBindGroup`] to auto-generate the GPU bind-group wiring.
/// 2. Point to a custom WGSL shader via `#[material(vertex_shader = …)]` using
///    `include_str!` (resolved relative to this source file).
/// 3. Implement [`Material`] — using the default methods is enough for an unlit
///    material: `needs_lighting()` and `needs_skeleton()` both return `false`, so
///    the engine only includes the camera bind group (`@group(1)`) in the pipeline
///    layout.  The WGSL source therefore never has to declare `@group(2)` or
///    `@group(3)`.
/// 4. Register [`MaterialPlugin::<UnlitMaterial>`] in the application.
/// 5. Attach `MaterialComponent::<UnlitMaterial>` to mesh entities instead of
///    the old `CustomMaterialComponent`.
use bytemuck::{Pod, Zeroable};
use essential::assets::Asset;
use render::{AsBindGroup, Material};

/// GPU-side uniform for [`UnlitMaterial`].
///
/// `repr(C)` with explicit padding to satisfy the 16-byte alignment required
/// by WebGPU uniform buffers.
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct TintUniform {
    /// RGBA tint colour applied to the whole mesh.
    pub color: [f32; 4],
    /// Explicit padding — keeps the struct at 32 bytes (two `vec4`s).
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
/// Attach it to a mesh entity via `MaterialComponent::<UnlitMaterial>`:
///
/// ```rust,ignore
/// let mat_handle = asset_server.add(UnlitMaterial {
///     tint: TintUniform::new(0.2, 0.8, 1.0, 1.0),
/// });
///
/// cmd.spawn((
///     MeshComponent { handle: mesh_handle },
///     MaterialComponent::<UnlitMaterial> { handle: mat_handle },
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

/// [`UnlitMaterial`] uses only the camera bind group.
/// Lighting and skeleton bindings are absent from the pipeline, so the WGSL
/// shader does not need to declare `@group(2)` or `@group(3)`.
impl Material for UnlitMaterial {
    fn needs_lighting() -> bool {
        false
    }
    fn needs_skeleton() -> bool {
        false
    }
}
