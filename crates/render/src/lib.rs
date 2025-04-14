#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

pub mod components;
pub mod mesh;
pub mod plugin;
pub mod render_pass;
pub mod resources;
pub mod systems;
pub mod wgpu_wrapper;

#[cfg(test)]
mod tests {}
