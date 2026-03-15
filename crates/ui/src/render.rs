use ecs::{
    query::{Query, change_detection::DetectChanges},
    resource::{Res, ResMut},
};
use glam::Mat4;
use glyphon::{Color, Resolution, TextArea, TextBounds};
use render::{device::RenderDevice, queue::RenderQueue, render_asset::render_window::RenderWindow};
use wgpu::util::{BufferInitDescriptor, DeviceExt};
use window::plugin::Window;

use crate::{
    layout::UICameraLayout,
    node::{RenderUIMaterial, RenderUINode},
    resources::UIRenderPipeline,
    text::{
        RenderTextComponent,
        resources::{TextAtlas, TextFontSystem, TextRenderer, TextSwashCache, TextViewport},
    },
};

pub(crate) fn update_text_viewport(
    window: Res<Window>,
    queue: Res<RenderQueue>,
    mut text_viewport: ResMut<TextViewport>,
) {
    if window.has_changed() {
        let size = window.size();
        text_viewport.update(
            &queue,
            Resolution {
                width: size.0,
                height: size.1,
            },
        );
    }
}

pub(crate) fn prepare_text_renderer(
    mut text_renderer: ResMut<TextRenderer>,
    mut font_system: ResMut<TextFontSystem>,
    mut text_atlas: ResMut<TextAtlas>,
    mut text_swash_cache: ResMut<TextSwashCache>,
    text_viewport: Res<TextViewport>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    text_nodes: Query<(&RenderUINode, &RenderTextComponent)>,
) {
    text_renderer
        .prepare(
            &device,
            &queue,
            &mut font_system,
            &mut text_atlas,
            &text_viewport,
            text_nodes
                .iter()
                .map(|(render_node, render_text)| TextArea {
                    buffer: &render_text.buffer,
                    left: render_node.location.x,
                    top: render_node.location.y,
                    scale: 1.0,
                    bounds: TextBounds {
                        left: 0,
                        top: 0,
                        right: render_node.size.x as i32,
                        bottom: 600 as i32,
                    },
                    default_color: Color::rgb(255, 255, 255),
                    custom_glyphs: &[],
                }),
            &mut text_swash_cache,
        )
        .expect("Failed preparing for rendering text");
}

pub(crate) fn ui_renderpass(
    pipeline: Res<UIRenderPipeline>,
    mut device: ResMut<RenderDevice>,
    render_window: Res<RenderWindow>,
    window: Res<Window>,
    ui_camera_layout: Res<UICameraLayout>,
    ui_nodes: Query<(&RenderUINode, &RenderUIMaterial)>,
    // Texture inputs
    text_renderer: ResMut<TextRenderer>,
    text_viewport: Res<TextViewport>,
    text_atlas: Res<TextAtlas>,
) {
    let projection_matrix = Mat4::orthographic_rh(
        0.0,
        window.width() as f32,
        window.height() as f32,
        0.0,
        0.0,
        1.0,
    );

    let ui_view = device.create_buffer_init(&BufferInitDescriptor {
        label: Some("UI Projection"),
        contents: bytemuck::cast_slice(&[projection_matrix]),
        usage: wgpu::BufferUsages::UNIFORM,
    });

    let ui_camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        layout: &ui_camera_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: ui_view.as_entire_binding(),
        }],
        label: Some("UI Camera Bind Group"),
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
        render_pass.set_bind_group(0, &ui_camera_bind_group, &[]);

        let mut render_nodes = ui_nodes.iter().collect::<Vec<_>>();
        render_nodes.sort_by_key(|(render_node, _)| render_node.z_index);
        for (render_node, render_material) in render_nodes {
            render_pass.set_index_buffer(
                render_node.index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );
            render_pass.set_vertex_buffer(0, render_node.vertex_buffer.slice(..));
            render_pass.set_bind_group(1, &render_material.material_bind_group, &[]);
            render_pass.draw_indexed(0..render_node.index_count, 0, 0..1);
        }

        // Render Text
        text_renderer
            .render(&text_atlas, &text_viewport, &mut render_pass)
            .expect("Error rendering text");
    }
}
