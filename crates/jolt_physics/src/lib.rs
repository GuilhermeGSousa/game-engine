//! A physics backend for the engine built on Jolt Physics via the raw
//! `joltc-sys` bindings, with a thin hand-written safe wrapper.

pub mod body;
pub mod collider;
pub mod physics_pipeline;
pub mod physics_state;
pub mod plugin;
pub mod rigid_body;

mod ffi_util;
mod init;
mod layers;
mod simulation;
