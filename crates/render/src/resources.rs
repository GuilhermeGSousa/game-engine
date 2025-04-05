use ecs::resource::Resource;
use std::sync::Arc;

use crate::{components::camera::CameraUniform, render_mesh::RenderMesh};

#[derive(Resource)]
pub struct RenderContext {
    pub(crate) surface: Arc<wgpu::Surface<'static>>,
    pub(crate) surface_config: wgpu::SurfaceConfiguration,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub pipeline: wgpu::RenderPipeline,

    // Probably temp
    pub diffuse_bind_group: wgpu::BindGroup,
    pub camera_bind_group: wgpu::BindGroup,
    pub camera_uniform: CameraUniform,
    pub camera_buffer: wgpu::Buffer,
}

#[derive(Resource)]
pub(crate) struct RenderWorldState {
    pub(crate) meshes: Vec<RenderMesh>,
}

impl RenderWorldState {
    pub fn new() -> Self {
        Self { meshes: Vec::new() }
    }

    pub fn add_mesh(&mut self, mesh: RenderMesh) {
        self.meshes.push(mesh);
    }

    pub fn clear(&mut self) {
        self.meshes.clear();
    }
}
