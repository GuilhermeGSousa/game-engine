pub mod assets;
pub mod components;
pub mod device;
pub mod layouts;
pub mod loaders;
pub mod plugin;
pub mod queue;
pub mod render_asset;
pub mod resources;
pub mod systems;
pub mod wgpu_wrapper;

/// Re-export the `AsBindGroup` derive macro so crates that depend on `render`
/// do not need to add `render-macros` as a separate dependency.
pub use render_macros::AsBindGroup;

#[cfg(test)]
mod tests {
    /// Verify that `#[derive(AsBindGroup)]` generates a valid `AsBindGroup`
    /// implementation.  This is a compile-time check only — no GPU device is
    /// required.
    #[test]
    fn as_bind_group_derive_compiles() {
        use crate::assets::{
            material::{AsBindGroup, ShaderRef},
            texture::Texture,
        };
        use bytemuck::{Pod, Zeroable};
        use essential::assets::{handle::AssetHandle, Asset};

        // A minimal custom material using the derive macro.
        #[derive(Asset, crate::AsBindGroup)]
        #[material(fragment_shader = "// empty shader")]
        struct ToonMaterial {
            #[texture(0)]
            albedo: Option<AssetHandle<Texture>>,
            #[sampler(1)]
            albedo_sampler: (),
            #[uniform(2)]
            toon_levels: ToonUniform,
        }

        #[repr(C)]
        #[derive(Copy, Clone, Pod, Zeroable)]
        struct ToonUniform {
            levels: f32,
            _pad: [f32; 3],
        }

        // Verify the generated ShaderRef values.
        assert_eq!(ToonMaterial::vertex_shader(), ShaderRef::Default);
        assert_eq!(
            ToonMaterial::fragment_shader(),
            ShaderRef::Source("// empty shader")
        );
    }
}
