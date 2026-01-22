use ecs::resource::{Res, ResMut};
use render::{
    device::RenderDevice, queue::RenderQueue, render_asset::render_window::RenderWindow,
    resources::RenderContext,
};
use wgpu::util::DeviceExt;

use crate::{
    resources::UIRenderPipeline,
    text::resources::{TextAtlas, TextFontSystem, TextRenderer, TextSwashCache, TextViewport},
    vertex::UIVertex,
};

pub(crate) fn ui_renderpass(
    context: Res<RenderContext>,
    pipeline: Res<UIRenderPipeline>,
    mut device: ResMut<RenderDevice>,
    render_window: Res<RenderWindow>,
    queue: Res<RenderQueue>,
    text_renderer: ResMut<TextRenderer>,
    font_system: ResMut<TextFontSystem>,
    text_atlas: ResMut<TextAtlas>,
    text_viewport: Res<TextViewport>,
    text_swash_cache: ResMut<TextSwashCache>,
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

        // Text Rendering (in the same pass)
        text_renderer
            .prepare(
                &device,
                &queue,
                &mut font_system,
                &mut text_atlas,
                &text_viewport,
                text_areas,
                &mut text_swash_cache,
            )
            .expect("Failed preparing for rendering text");

        render_pass.set_pipeline(&pipeline);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.draw(0..3, 0..1);
    }
}
