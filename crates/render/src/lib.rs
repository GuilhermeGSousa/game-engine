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

/// Re-export the `Material` trait and plugin types for convenience.
pub use assets::material::Material;
pub use components::material_component::MaterialComponent;
pub use material_plugin::{MaterialPipeline, MaterialPlugin, RenderMaterial};
