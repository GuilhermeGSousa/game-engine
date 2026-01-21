use ecs::resource::{Res, ResMut};
use render::{device::RenderDevice, resources::RenderContext};
use wgpu::util::DeviceExt;

use crate::{resources::UIRenderPipeline, vertex::UIVertex};

pub(crate) fn ui_renderpass(
    context: Res<RenderContext>,
    pipeline: Res<UIRenderPipeline>,
    mut device: ResMut<RenderDevice>,
) {
    let encoder = device.command_encoder();

    let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
        label: Some("UI Render Pass"),
        color_attachments: &[],
        depth_stencil_attachment: None,
        occlusion_query_set: None,
        timestamp_writes: None,
    });

    let mut vertices = Vec::new();
    vertices.push(UIVertex {
        pos_coords: todo!(),
    });
    render_pass.set_pipeline(&pipeline);
    let vertices = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("UI Vertex Buffer"),
        contents: todo!(),
        usage: wgpu::BufferUsages::VERTEX,
    });
    // render_pass.set_vertex_buffer()
}
