use ecs::{query::change_detection::DetectChanges, resource::{Res, ResMut}};
use glam::Mat4;
use glyphon::{Attrs, Buffer, Color, Family, Metrics, Resolution, Shaping, TextArea, TextBounds};
use render::{
    device::RenderDevice, queue::RenderQueue, render_asset::render_window::RenderWindow,
};
use wgpu::util::DeviceExt;
use window::plugin::Window;

use crate::{
    resources::UIRenderPipeline, text::resources::{TextAtlas, TextFontSystem, TextRenderer, TextSwashCache, TextViewport}, transform::UIGlobalTransform, vertex::UIVertex
};

// pub(crate) fn camera_added(
//     cameras: Query<(Entity, &Camera, &GlobalTranform, Option<&RenderEntity>), Added<(Camera,)>>,
//     mut cmd: CommandQueue,
//     device: Res<RenderDevice>,
//     context: Res<RenderContext>,
// )
// {
//     let projection_matrix = Mat4::orthographic_rh(
//                 0.0,
//                 physical_viewport_rect.width() as f32,
//                 physical_viewport_rect.height() as f32,
//                 0.0,
//                 0.0,
//                 1.0,
//             );

// }


pub(crate) fn update_text_viewport(
    window: Res<Window>,
    queue: Res<RenderQueue>,
    mut text_viewport: ResMut<TextViewport>,)
{
    if window.has_changed()
    {
        let size = window.size();
        text_viewport.update(&queue, 
            Resolution { width: size.0, height: size.1 });
    }
}

pub(crate) fn ui_renderpass(
    pipeline: Res<UIRenderPipeline>,
    mut device: ResMut<RenderDevice>,
    render_window: Res<RenderWindow>,
    queue: Res<RenderQueue>,
    mut text_renderer: ResMut<TextRenderer>,
    mut font_system: ResMut<TextFontSystem>,
    mut text_atlas: ResMut<TextAtlas>,
    text_viewport: Res<TextViewport>,
    mut text_swash_cache: ResMut<TextSwashCache>,
) {

    let mut vertices = Vec::new();
    vertices.push(UIVertex {
        pos_coords: [0.0, 0.0],
    });
    vertices.push(UIVertex {
        pos_coords: [0.5, 0.5],
    });
    vertices.push(UIVertex {
        pos_coords: [0.0, 0.5],
    });

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("UI Vertex Buffer"),
        contents: bytemuck::cast_slice(&vertices),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let transform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("UI Vertex Buffer"),
        contents: bytemuck::cast_slice(&[*UIGlobalTransform::default()]),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let mut test_text_buffer = Buffer::new(&mut font_system, Metrics::new(30.0, 42.0));
    test_text_buffer.set_size(&mut font_system, Some(1000.0), Some(1000.0));
    test_text_buffer.set_text(
        &mut font_system, 
        "Hello world! 👋\nThis is rendered with 🦅 glyphon 🦁\nThe text below should be partially clipped.\na b c d e f g h i j k l m n o p q r s t u v w x y z", 
        Attrs::new().family(Family::SansSerif), 
        Shaping::Advanced);
    test_text_buffer.shape_until_scroll(&mut font_system, false);
    text_renderer
        .prepare(
            &device,
            &queue,
            &mut font_system,
            &mut text_atlas,
            &text_viewport,
            [TextArea {
                buffer: &test_text_buffer,
                left: 10.0,
                top: 10.0,
                scale: 1.0,
                bounds: TextBounds {
                    left: 0,
                    top: 0,
                    right: 600,
                    bottom: 160,
                },
                default_color: Color::rgb(255, 255, 255),
                custom_glyphs: &[],
            }],
            &mut text_swash_cache,
        )
        .expect("Failed preparing for rendering text");

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

        render_pass.set_pipeline(&pipeline);
        render_pass.set_vertex_buffer(0, vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, transform_buffer.slice(..));
        render_pass.draw(0..3, 0..1);

        text_renderer.render(&text_atlas, &text_viewport, &mut render_pass).unwrap();
    }
}