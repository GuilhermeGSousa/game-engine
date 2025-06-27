use ecs::resource::Resource;
use std::sync::Arc;

use crate::{components::camera::CameraUniform, render_asset::render_texture::RenderTexture};

#[derive(Resource)]
pub struct RenderContext {
    pub(crate) surface: Arc<wgpu::Surface<'static>>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub pipeline: wgpu::RenderPipeline,

    // Probably temp
    pub camera_bind_group: wgpu::BindGroup,
    pub camera_uniform: CameraUniform,
    pub camera_buffer: wgpu::Buffer,
    pub(crate) depth_texture: RenderTexture,
}
