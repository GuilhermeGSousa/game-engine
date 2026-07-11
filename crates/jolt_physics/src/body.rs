use ecs::Component;
use jolt_ffi::JoltBodyId;

/// A handle to a body living inside the Jolt physics system.
///
/// This is a plain `u32`-backed value (Jolt's body id), so it is trivially
/// `Copy` and `Send + Sync` — no raw pointers are stored in the ECS components.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Component)]
pub struct BodyId(pub(crate) JoltBodyId);
