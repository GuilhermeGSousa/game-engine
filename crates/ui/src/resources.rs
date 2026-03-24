use std::ops::Deref;

use ecs::resource::Resource;

/// The UI render pipeline together with the `@group(1)` bind-group layout for
/// `UIMaterial` (the colour uniform).  Both are created in `UIPlugin::finish`
/// and kept together so that `extract_added_ui_materials` can access the layout
/// without needing a separate resource.
#[derive(Resource)]
pub(crate) struct UIRenderPipeline {
    pub(crate) pipeline: wgpu::RenderPipeline,
    /// `@group(1)` bind-group layout for `UIMaterial`.
    pub(crate) material_layout: wgpu::BindGroupLayout,
}

impl UIRenderPipeline {
    pub fn new(pipeline: wgpu::RenderPipeline, material_layout: wgpu::BindGroupLayout) -> Self {
        Self { pipeline, material_layout }
    }
}

impl Deref for UIRenderPipeline {
    type Target = wgpu::RenderPipeline;

    fn deref(&self) -> &Self::Target {
        &self.pipeline
    }
}
