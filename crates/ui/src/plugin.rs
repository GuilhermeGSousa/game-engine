use app::plugins::Plugin;
use render::{assets::vertex::VertexBufferLayout, device::RenderDevice, resources::RenderContext};
use wgpu::PipelineLayoutDescriptor;

use crate::{render::ui_renderpass, resources::UIRenderPipeline, vertex::UIVertex};

pub struct UIPlugin;

impl Plugin for UIPlugin {
    fn build(&self, app: &mut app::App) {
        app.add_system(app::update_group::UpdateGroup::Render, ui_renderpass);
    }

    fn finish(&self, app: &mut app::App) {
        let device = app
            .get_resource::<RenderDevice>()
            .expect("RenderDevice resource not found");

        let context = app
            .get_resource::<RenderContext>()
            .expect("RenderContext resource not found");

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

        app.insert_resource(UIRenderPipeline::new(ui_render_pipeline));
    }
}
