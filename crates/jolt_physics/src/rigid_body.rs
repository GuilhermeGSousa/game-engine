use ecs::component::Component;

/// Marks an entity's [`Collider`](crate::collider::Collider) as a dynamic
/// body and describes its dynamics.
///
/// The Jolt body itself is created by `Collider`'s lifecycle: spawn
/// `RigidBody` in the same bundle as (or before) the `Collider` so it is
/// visible when the body is created. Fields are read at that moment only —
/// changing them afterwards does not affect the live body.
#[derive(Component, Clone, Copy, Debug)]
pub struct RigidBody {
    /// Density in kg/m³; the body's mass is the collider shape's volume times
    /// this. Defaults to 1000 (water).
    pub density: f32,
}

impl RigidBody {
    pub fn with_density(density: f32) -> Self {
        Self { density }
    }
}

impl Default for RigidBody {
    fn default() -> Self {
        Self { density: 1000.0 }
    }
}
