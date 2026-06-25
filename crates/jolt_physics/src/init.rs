use std::sync::OnceLock;

use joltc_sys::{JPC_FactoryInit, JPC_RegisterDefaultAllocator, JPC_RegisterTypes};

/// Performs Jolt's process-global, one-time initialisation.
///
/// Jolt requires the default allocator, factory, and type registry to be set up
/// exactly once before any `JPC_PhysicsSystem` is created. This is safe to call
/// from multiple [`PhysicsState`](crate::physics_state::PhysicsState)s: the work
/// runs only on the first call thanks to the [`OnceLock`].
pub(crate) fn global_init() {
    static INITIALIZED: OnceLock<()> = OnceLock::new();

    INITIALIZED.get_or_init(|| unsafe {
        JPC_RegisterDefaultAllocator();
        JPC_FactoryInit();
        JPC_RegisterTypes();
    });
}
