use core::resource::Resource;
use std::sync::Arc;

use crate::vertex;

#[derive(Resource)]
pub(crate) struct RenderSurface(pub(crate) Arc<wgpu::Surface<'static>>);

#[derive(Resource)]
pub(crate) struct RenderAdapter(pub(crate) wgpu::Adapter);

#[derive(Resource)]
pub(crate) struct RenderDevice(pub(crate) wgpu::Device);

#[derive(Resource)]
pub(crate) struct RenderQueue(pub(crate) wgpu::Queue);

#[derive(Resource)]
pub(crate) struct RenderConfig(pub(crate) wgpu::SurfaceConfiguration);

#[derive(Resource)]
pub(crate) struct RenderPipeline(pub(crate) wgpu::RenderPipeline);

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
