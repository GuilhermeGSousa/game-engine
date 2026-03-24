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

/// The skybox render pipeline together with the `@group(0)` bind-group layout
/// for the skybox material.  Both are created together in `RenderPlugin::finish`
/// and the layout is kept here so that `prepare_skybox` can use it to build
/// the per-camera bind group without creating a new layout object each frame.
#[derive(Resource)]
pub(crate) struct SkyboxRenderPipeline {
    pub(crate) pipeline: wgpu::RenderPipeline,
    /// `@group(0)` bind-group layout for `SkyboxMaterial` (cube texture + sampler).
    pub(crate) material_layout: wgpu::BindGroupLayout,
}

impl Deref for SkyboxRenderPipeline {
    type Target = wgpu::RenderPipeline;

    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}

impl SkyboxRenderPipeline {
    pub fn new(pipeline: wgpu::RenderPipeline, material_layout: wgpu::BindGroupLayout) -> Self {
        Self { pipeline, material_layout }
    }
}
