//! Physics for the engine: backend-agnostic components, resources, and
//! systems over a compile-time-selected physics engine.
//!
//! The engine-facing API is shared code; everything backend-specific hides
//! behind the [`backend::PhysicsBackend`] trait. Native builds use Jolt
//! Physics (via the in-repo `jolt-ffi` bindings).

pub mod backend;
pub mod body;
pub mod collider;
pub mod ground;
pub mod interpolation;
pub mod movement;
pub mod physics_pipeline;
pub mod physics_state;
pub mod plugin;
pub mod ray;
pub mod rigid_body;

mod simulation;

/// The physics engine compiled into this build. Exactly one exists per
/// target; the alias is the only place the choice is made.
pub type ActiveBackend = backend::jolt::JoltBackend;
