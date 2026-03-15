use std::ops::Deref;

use ecs::resource::Resource;

#[derive(Resource)]
pub(crate) struct UIRenderPipeline(wgpu::RenderPipeline);

impl UIRenderPipeline {
    pub fn new(pipeline: wgpu::RenderPipeline) -> Self {
        Self(pipeline)
    }
}

impl Deref for UIRenderPipeline {
    type Target = wgpu::RenderPipeline;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}
