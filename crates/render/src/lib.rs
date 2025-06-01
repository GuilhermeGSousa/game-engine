#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

pub mod assets;
pub mod components;
pub mod layouts;
pub mod loaders;
pub mod plugin;
pub mod render_asset;
pub mod resources;
pub mod systems;
pub mod wgpu_wrapper;

#[cfg(test)]
mod tests {}
