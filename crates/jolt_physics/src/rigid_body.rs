use std::ops::{Deref, DerefMut};

use ecs::component::Component;
use essential::transform::Transform;
use joltc_sys::*;

use crate::body::BodyId;
use crate::ffi_util::{create_sphere, rvec3};
use crate::layers::OL_MOVING;
use crate::physics_state::PhysicsState;

/// A dynamic rigid body. Wraps the Jolt [`BodyId`]; attach a shape to it with
/// [`PhysicsState::make_sphere`](crate::physics_state::PhysicsState::make_sphere)
/// or [`make_cuboid`](crate::physics_state::PhysicsState::make_cuboid).
#[derive(Component)]
pub struct RigidBody(BodyId);

impl RigidBody {
    /// Creates a dynamic body at `transform`'s position and adds it to the
    /// simulation.
    ///
    /// Jolt requires a shape at body-creation time, so the body starts with a
    /// small placeholder sphere. A subsequent `make_sphere`/`make_cuboid` call
    /// replaces it with the real collider via `SetShape`.
    pub fn new(transform: &Transform, state: &mut PhysicsState) -> Self {
        let placeholder = create_sphere(0.5);

        let settings = JPC_BodyCreationSettings {
            Position: rvec3(transform.translation),
            MotionType: JPC_MOTION_TYPE_DYNAMIC,
            ObjectLayer: OL_MOVING,
            Shape: placeholder,
            ..Default::default()
        };

        // SAFETY: the body interface is valid for the lifetime of `state`, and
        // `settings` is fully initialised with a valid shape pointer.
        let id = unsafe {
            JPC_BodyInterface_CreateAndAddBody(
                state.body_interface(),
                &settings,
                JPC_ACTIVATION_ACTIVATE,
            )
        };

        Self(BodyId(id))
    }
}

impl Deref for RigidBody {
    type Target = BodyId;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for RigidBody {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}
