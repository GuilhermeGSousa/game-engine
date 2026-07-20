use std::sync::Arc;

use mesh::Mesh;

use crate::{backend::PhysicsBackend, ActiveBackend};

pub struct PhysicsShape(pub(crate) <ActiveBackend as PhysicsBackend>::ShapeHandle);

impl PhysicsShape {
    pub fn from_mesh(mesh: &Mesh) -> Self {
        Self(<ActiveBackend as PhysicsBackend>::create_shape_from_mesh(
            mesh,
        ))
    }

    pub fn create_sphere_shape(radius: f32) -> Self {
        Self(ActiveBackend::create_sphere_shape(radius))
    }

    pub fn create_cuboid_shape(width: f32, height: f32, length: f32) -> Self {
        Self(ActiveBackend::create_cuboid_shape(width, height, length))
    }

    pub fn create_capsule_shape(half_height: f32, radius: f32) -> Self {
        Self(ActiveBackend::create_capsule_shape(half_height, radius))
    }
}

pub type SharedPhysicsShape = Arc<PhysicsShape>;
