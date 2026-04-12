use ecs::{
    query::{Query, change_detection::DetectChanges},
    resource::{Res, ResMut},
};
use glyphon::{Color, Resolution, TextArea, TextBounds};
use render::{
    MaterialPipeline, device::RenderDevice, queue::RenderQueue,
    render_asset::render_window::RenderWindow,
};

use window::plugin::Window;

use crate::{
    material::UIMaterial,
    node::{RenderUIMaterial, RenderUINode, RenderUIViewport, UIViewportPipeline},
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

/// Compute the screen-space scissor rectangle for a text node.
///
/// `TextBounds` is a clip rect in **absolute screen pixel coordinates** — all
/// four values are measured from the top-left corner of the window, not from
/// the node's own origin.  The text that falls outside this rect is discarded
/// by glyphon before it reaches the GPU.
///
/// The node's `location` is already in absolute screen pixels (set by
/// `write_absolute_positions` in `node.rs`), so we just map it straight
/// through.
fn node_text_bounds(location: glam::Vec2, size: glam::Vec2) -> TextBounds {
    TextBounds {
        left: location.x as i32,
        top: location.y as i32,
        right: (location.x + size.x) as i32,
        bottom: (location.y + size.y) as i32,
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
                    bounds: node_text_bounds(render_node.location, render_node.size),
                    default_color: Color::rgb(255, 255, 255),
                    custom_glyphs: &[],
                }),
            &mut text_swash_cache,
        )
        .expect("Failed preparing for rendering text");
}

#[cfg(test)]
mod tests {
    use super::node_text_bounds;
    use glam::Vec2;

    #[test]
    fn bounds_match_node_absolute_position() {
        let loc = Vec2::new(10.0, 800.0);
        let size = Vec2::new(120.0, 32.0);
        let b = node_text_bounds(loc, size);
        assert_eq!(b.left, 10);
        assert_eq!(b.top, 800);
        assert_eq!(b.right, 130); // 10 + 120
        assert_eq!(b.bottom, 832); // 800 + 32
    }

    #[test]
    fn bounds_at_origin() {
        let b = node_text_bounds(Vec2::ZERO, Vec2::new(800.0, 600.0));
        assert_eq!(b.left, 0);
        assert_eq!(b.top, 0);
        assert_eq!(b.right, 800);
        assert_eq!(b.bottom, 600);
    }
}

pub(crate) fn ui_renderpass(
    pipeline: Res<MaterialPipeline<UIMaterial>>,
    viewport_pipeline: Res<UIViewportPipeline>,
    mut device: ResMut<RenderDevice>,
    render_window: Res<RenderWindow>,
    ui_nodes: Query<(
        &RenderUINode,
        Option<&RenderUIMaterial>,
        Option<&RenderUIViewport>,
    )>,
    // Text
    text_renderer: ResMut<TextRenderer>,
    text_viewport: Res<TextViewport>,
    text_atlas: Res<TextAtlas>,
) {
    let encoder = device.command_encoder();

    if let Some(view) = render_window.get_view() {
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("UI Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view,
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

        let mut render_nodes = ui_nodes.iter().collect::<Vec<_>>();
        render_nodes.sort_by_key(|(render_node, _, _)| render_node.z_index);

        for (render_node, render_material, render_viewport) in render_nodes {
            render_pass.set_index_buffer(
                render_node.index_buffer.slice(..),
                wgpu::IndexFormat::Uint16,
            );
            render_pass.set_vertex_buffer(0, render_node.vertex_buffer.slice(..));

            if let Some(viewport) = render_viewport {
                render_pass.set_pipeline(&viewport_pipeline.pipeline);
                render_pass.set_bind_group(0, &viewport.bind_group, &[]);
            } else if let Some(material) = render_material {
                render_pass.set_pipeline(&pipeline.pipeline);
                render_pass.set_bind_group(0, &material.material_bind_group, &[]);
            } else {
                continue;
            }

            render_pass.draw_indexed(0..render_node.index_count, 0, 0..1);
        }

        // Render Text
        text_renderer
            .render(&text_atlas, &text_viewport, &mut render_pass)
            .expect("Error rendering text");
    }
}
