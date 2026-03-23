pub mod assets;
pub mod components;
pub mod device;
pub mod layouts;
pub mod loaders;
pub mod material_plugin;
pub mod plugin;
pub mod queue;
pub mod render_asset;
pub mod resources;
pub mod systems;
pub mod wgpu_wrapper;

/// Re-export the `AsBindGroup` derive macro so crates that depend on `render`
/// do not need to add `render-macros` as a separate dependency.
pub use render_macros::AsBindGroup;

/// Re-export the material plugin and related types for convenience.
pub use material_plugin::{
    CustomMaterialComponent, MaterialPlugin, MaterialPipeline, RenderCustomMaterial,
};
