use encase::ShaderType;
use essential::{assets::handle::AssetHandle, transform::GlobalTranform};

use ecs::{component::Component, entity::Entity};
use glam::{Mat4, Vec3};

use crate::{assets::texture::Texture, render_asset::render_texture::RenderTexture};

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: Mat4 = Mat4::from_cols_array(
    &[
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0
    ]
);

pub enum WindowRef {
    MainWindow,
    CustomWindow(Entity),
}

#[allow(dead_code)]
pub enum RenderTarget {
    Window(WindowRef),
    Texture(AssetHandle<Texture>),
}

#[allow(dead_code)]
impl RenderTarget {
    pub fn main_window() -> Self {
        RenderTarget::Window(WindowRef::MainWindow)
    }

    pub fn custom_window(entity: Entity) -> Self {
        RenderTarget::Window(WindowRef::CustomWindow(entity))
    }

    pub fn texture(handle: AssetHandle<Texture>) -> Self {
        RenderTarget::Texture(handle)
    }
}

#[derive(Component)]
pub struct Camera {
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
    pub clear_color: wgpu::Color,
    pub render_target: RenderTarget,
}

impl Camera {
    pub fn build_projection_matrix(&self) -> Mat4 {
        Mat4::perspective_rh(self.fovy.to_radians(), self.aspect, self.znear, self.zfar)
    }
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            aspect: 1.0,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
            clear_color: wgpu::Color {
                r: 0.118,
                g: 0.831,
                b: 0.922,
                a: 1.0,
            },
            render_target: RenderTarget::main_window(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, ShaderType)]
pub struct CameraUniform {
    view_pos: Vec3,
    view_proj: Mat4,
}

impl CameraUniform {
    pub fn new() -> Self {
        Self {
            view_pos: Vec3::ZERO,
            view_proj: Mat4::IDENTITY,
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera, transform: &GlobalTranform) {
        self.view_pos = transform.translation().into();
        self.view_proj =
            OPENGL_TO_WGPU_MATRIX * camera.build_projection_matrix() * transform.matrix().inverse();
    }
}

#[derive(Component)]
pub struct RenderCamera {
    pub(crate) clear_color: wgpu::Color,
    pub camera_bind_group: wgpu::BindGroup,
    pub camera_uniform: CameraUniform,
    pub camera_buffer: wgpu::Buffer,
    pub(crate) depth_texture: RenderTexture,
}
