use ecs::component::Component;

use super::transform::Transform;

#[derive(Component)]
pub struct Camera {
    aspect: f32,
    fovy: f32,
    znear: f32,
    zfar: f32,
}

impl Camera {
    pub fn new(aspect: f32, fovy: f32, znear: f32, zfar: f32) -> Self {
        Self {
            aspect: aspect,
            fovy: fovy,
            znear: znear,
            zfar: zfar,
        }
    }

    pub fn projection_matrix(&self) -> Transform {
        Transform::from_matrix(cgmath::perspective(
            cgmath::Deg(self.fovy),
            self.aspect,
            self.znear,
            self.zfar,
        ))
    }
}
