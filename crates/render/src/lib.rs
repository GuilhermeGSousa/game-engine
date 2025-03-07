#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

pub mod plugin;
pub mod systems;
pub mod vertex;
#[cfg(test)]
mod tests {}
