use app::plugins::Plugin;
use glyphon::{Cache, FontSystem, SwashCache, Viewport};
use render::{
    assets::vertex::VertexBufferLayout, device::RenderDevice, queue::RenderQueue,
    resources::RenderContext,
};
use wgpu::{MultisampleState, PipelineLayoutDescriptor};

use crate::{
    render::{ui_renderpass, update_text_viewport},
    resources::UIRenderPipeline,
    text::resources::{
        TextAtlas, TextCache, TextFontSystem, TextRenderer, TextSwashCache, TextViewport,
    },
    vertex::UIVertex,
};

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut app::App) {
        app.add_system(app::update_group::UpdateGroup::Render, update_text_viewport)
            .add_system(app::update_group::UpdateGroup::Render, ui_renderpass);
    }

    fn finish(&self, app: &mut app::App) {
        let device = app
            .get_resource::<RenderDevice>()
            .expect("RenderDevice resource not found");

        let context = app
            .get_resource::<RenderContext>()
            .expect("RenderContext resource not found");

        let queue = app
            .get_resource::<RenderQueue>()
            .expect("RenderQueue resource not found");

        // Text rendering
        let font_system = FontSystem::new();
        let swash_cache = SwashCache::new();
        let cache = Cache::new(&device);
        let viewport = Viewport::new(&device, &cache);
        // TODO: Update Viewport on resize!

        let mut atlas =
            glyphon::TextAtlas::new(&device, &queue, &cache, context.surface_config.format);
        let text_renderer =
            glyphon::TextRenderer::new(&mut atlas, &device, MultisampleState::default(), None);

        // UI rendering
        let ui_shader = device.create_shader_module(wgpu::include_wgsl!("shaders\\ui.wgsl"));

        let ui_render_pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("UI Render Pipeline Layout"),
            bind_group_layouts: &[],
            push_constant_ranges: &[],
        });

        let ui_render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("UI Pipeline"),
            layout: Some(&ui_render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &ui_shader,
                entry_point: Some("vs_main"),
                buffers: &[UIVertex::describe()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &ui_shader,
                entry_point: Some("fs_main"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: context.surface_config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        app.insert_resource(UIRenderPipeline::new(ui_render_pipeline))
            .insert_resource(TextRenderer(text_renderer))
            .insert_resource(TextCache(cache))
            .insert_resource(TextSwashCache(swash_cache))
            .insert_resource(TextViewport(viewport))
            .insert_resource(TextFontSystem(font_system))
            .insert_resource(TextAtlas(atlas));
    }
}
