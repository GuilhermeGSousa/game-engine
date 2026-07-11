use std::ops::BitOr;

use ecs::component::Component;

/// Bitmask of the degrees of freedom a dynamic body may use (combine with
/// `|`). Restricting a body to [`AllowedDofs::TRANSLATION`] keeps it from
/// ever rotating — e.g. a player capsule that must stay upright.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct AllowedDofs(pub(crate) jolt_ffi::JoltAllowedDofs);

impl AllowedDofs {
    pub const TRANSLATION_X: Self = Self(jolt_ffi::JOLT_ALLOWED_DOFS_TRANSLATION_X);
    pub const TRANSLATION_Y: Self = Self(jolt_ffi::JOLT_ALLOWED_DOFS_TRANSLATION_Y);
    pub const TRANSLATION_Z: Self = Self(jolt_ffi::JOLT_ALLOWED_DOFS_TRANSLATION_Z);
    pub const ROTATION_X: Self = Self(jolt_ffi::JOLT_ALLOWED_DOFS_ROTATION_X);
    pub const ROTATION_Y: Self = Self(jolt_ffi::JOLT_ALLOWED_DOFS_ROTATION_Y);
    pub const ROTATION_Z: Self = Self(jolt_ffi::JOLT_ALLOWED_DOFS_ROTATION_Z);
    /// All translation axes, no rotation.
    pub const TRANSLATION: Self = Self(
        jolt_ffi::JOLT_ALLOWED_DOFS_TRANSLATION_X
            | jolt_ffi::JOLT_ALLOWED_DOFS_TRANSLATION_Y
            | jolt_ffi::JOLT_ALLOWED_DOFS_TRANSLATION_Z,
    );
    /// All rotation axes, no translation.
    pub const ROTATION: Self = Self(
        jolt_ffi::JOLT_ALLOWED_DOFS_ROTATION_X
            | jolt_ffi::JOLT_ALLOWED_DOFS_ROTATION_Y
            | jolt_ffi::JOLT_ALLOWED_DOFS_ROTATION_Z,
    );
    pub const ALL: Self = Self(jolt_ffi::JOLT_ALLOWED_DOFS_ALL);
}

impl BitOr for AllowedDofs {
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self(self.0 | rhs.0)
    }
}

impl Default for AllowedDofs {
    fn default() -> Self {
        Self::ALL
    }
}

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
    pub allowed_dofs: AllowedDofs,
}

impl RigidBody {
    pub fn with_density(density: f32) -> Self {
        Self {
            density,
            ..Self::default()
        }
    }
}

impl Default for RigidBody {
    fn default() -> Self {
        Self {
            density: 1000.0,
            allowed_dofs: AllowedDofs::ALL,
        }
    }
}
