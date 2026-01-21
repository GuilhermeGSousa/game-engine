use ecs::resource::{Res, ResMut};
use render::{
    device::RenderDevice, render_asset::render_window::RenderWindow, resources::RenderContext,
};
use wgpu::util::DeviceExt;

use crate::{resources::UIRenderPipeline, vertex::UIVertex};

pub(crate) fn ui_renderpass(
    context: Res<RenderContext>,
    pipeline: Res<UIRenderPipeline>,
    mut device: ResMut<RenderDevice>,
    render_window: Res<RenderWindow>,
) {
    let mut vertices = Vec::new();
    vertices.push(UIVertex {
        pos_coords: [0.0, 0.0],
    });
    vertices.push(UIVertex {
        pos_coords: [1.0, 1.0],
    });
    vertices.push(UIVertex {
        pos_coords: [0.0, 1.0],
    });
    
    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("UI Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let encoder = device.command_encoder();

    if let Some(view) = render_window.get_view() {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("UI Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Load,
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            occlusion_query_set: None,
            timestamp_writes: None,
        });

        render_pass.set_pipeline(&pipeline);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.draw(0..3, 0..1);
    }
}
