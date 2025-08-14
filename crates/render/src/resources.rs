use ecs::resource::Resource;
use std::sync::Arc;

#[derive(Resource)]
pub struct RenderContext {
    pub(crate) surface: Arc<wgpu::Surface<'static>>,
    pub surface_config: wgpu::SurfaceConfiguration,
    pub main_pipeline: wgpu::RenderPipeline,
    pub skybox_pipeline: wgpu::RenderPipeline,
}
