use ecs::resource::Resource;
use std::sync::Arc;

#[derive(Resource)]
pub struct RenderContext {
    pub(crate) surface: Arc<wgpu::Surface<'static>>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub pipeline: wgpu::RenderPipeline,
}
