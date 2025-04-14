use essential::transform::Transform;

use bytemuck::{Pod, Zeroable};
use ecs::component::Component;
use glam::Mat4;

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: Mat4 = Mat4::from_cols_array(
    &[
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0
    ]
);

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

    pub fn build_projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fovy.to_radians(), self.aspect, self.znear, self.zfar)
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_proj: Mat4::IDENTITY.to_cols_array_2d(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera, transform: &Transform) {
        self.view_proj = (OPENGL_TO_WGPU_MATRIX
            * camera.build_projection_matrix()
            * transform.compute_matrix().inverse())
        .to_cols_array_2d();
    }
}

unsafe impl Pod for CameraUniform {}
unsafe impl Zeroable for CameraUniform {}
