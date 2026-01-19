use ecs::resource::Resource;
use std::{ops::Deref, sync::Arc};

#[derive(Resource)]
pub struct RenderContext {
    pub(crate) surface: Arc<wgpu::Surface<'static>>,
    pub surface_config: wgpu::SurfaceConfiguration,
}

#[derive(Resource)]
pub(crate) struct MainRenderPipeline(wgpu::RenderPipeline);

impl MainRenderPipeline {
    pub fn new(pipeline: wgpu::RenderPipeline) -> Self {
        Self(pipeline)
    }
}

impl Deref for MainRenderPipeline {
    type Target = wgpu::RenderPipeline;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Resource)]
pub(crate) struct SkyboxRenderPipeline(wgpu::RenderPipeline);

impl Deref for SkyboxRenderPipeline {
    type Target = wgpu::RenderPipeline;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl SkyboxRenderPipeline {
    pub fn new(pipeline: wgpu::RenderPipeline) -> Self {
        Self(pipeline)
    }
}
