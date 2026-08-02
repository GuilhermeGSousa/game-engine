use app::Plugin;
use ecs::Resource;
use essential::transform::GlobalTransformRaw;
use mesh::Vertex;
use wgpu::{
    BindGroupLayoutDescriptor, BindGroupLayoutEntry, DepthBiasState, DepthStencilState,
    FragmentState, MultisampleState, PipelineCompilationOptions, PipelineLayoutDescriptor,
    PrimitiveState, RenderPipelineDescriptor, ShaderModuleDescriptor, StencilState, TextureFormat,
};

use crate::{assets::vertex::VertexBufferLayout, device::RenderDevice, resources::RenderContext};

#[derive(Resource)]
pub(crate) struct ShadowPipeline {
    pub(crate) pipeline: wgpu::RenderPipeline,
    pub(crate) bind_group_layout: wgpu::BindGroupLayout,
}

pub struct ShadowPipelinePlugin;

impl Plugin for ShadowPipelinePlugin {
    fn build(&self, app: &mut app::App) {}

    fn finish(&self, app: &mut app::App) {
        let device = app
            .get_resource::<RenderDevice>()
            .expect("RenderDevice not found; register RenderPlugin before MaterialPlugin");

        let vs_module = device.create_shader_module(ShaderModuleDescriptor {
            label: Some("Shadows VS"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shaders/shadow.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&BindGroupLayoutDescriptor {
            label: Some("Shadow Bind Group Layout"),
            entries: &vec![BindGroupLayoutEntry {
                binding: todo!(),
                visibility: todo!(),
                ty: todo!(),
                count: todo!(),
            }],
        });

        let pipeline_layout = device.create_pipeline_layout(&PipelineLayoutDescriptor {
            label: Some("Shadow Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&RenderPipelineDescriptor {
            label: Some("Shadow Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &vs_module,
                entry_point: Some("vs_main"),
                buffers: &vec![Vertex::describe(), GlobalTransformRaw::describe()],
                compilation_options: PipelineCompilationOptions::default(),
            },
            fragment: None,
            primitive: PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                unclipped_depth: false,
                polygon_mode: wgpu::PolygonMode::Fill,
                conservative: false,
            },
            depth_stencil: Some(DepthStencilState {
                format: TextureFormat::Depth32Float,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: StencilState::default(),
                bias: DepthBiasState::default(),
            }),
            multisample: MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        app.insert_resource(ShadowPipeline {
            pipeline,
            bind_group_layout,
        });
    }
}
