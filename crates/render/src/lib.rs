#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

pub mod plugin;

#[cfg(test)]
mod tests {}
