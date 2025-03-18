use core::resource::Resource;
use std::sync::Arc;

#[derive(Resource)]
pub struct RenderSurface(pub(crate) Arc<wgpu::Surface<'static>>);

#[derive(Resource)]
pub struct RenderDevice(pub wgpu::Device);

#[derive(Resource)]
pub struct RenderQueue(pub wgpu::Queue);

#[derive(Resource)]
pub struct RenderConfig(pub(crate) wgpu::SurfaceConfiguration);

#[derive(Resource)]
pub struct RenderPipeline(pub(crate) wgpu::RenderPipeline);

#[derive(Resource)]
pub(crate) struct RenderBuffer {
    pub(crate) vertex_buffer: wgpu::Buffer,
    pub(crate) index_buffer: wgpu::Buffer,
}

impl RenderBuffer {
    pub(crate) fn new(vertex_buffer: wgpu::Buffer, index_buffer: wgpu::Buffer) -> Self {
        Self {
            vertex_buffer,
            index_buffer,
        }
    }
}

#[derive(Resource)]
pub struct RenderDifuseBindGroup(pub(crate) wgpu::BindGroup);
