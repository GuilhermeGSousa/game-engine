use ecs::resource::{Res, ResMut};
use egui_wgpu::{
    wgpu::{self, StoreOp},
    ScreenDescriptor,
};
use render::{render_asset::render_window::RenderWindow, resources::RenderContext};
use window::plugin::Window;

use crate::resources::UIRenderer;

pub(crate) fn begin_ui_frame(
    mut ui_renderer: ResMut<UIRenderer>,
    window: Res<Window>,
    render_window: Res<RenderWindow>,
) {
    if let Some(_) = render_window.get_view() {
        let raw_input = ui_renderer.state.take_egui_input(&window.window_handle);
        ui_renderer.state.egui_ctx().begin_pass(raw_input);
    }
}

pub(crate) fn end_ui_frame(
    mut ui_renderer: ResMut<UIRenderer>,
    render_context: Res<RenderContext>,
    window: Res<Window>,
    render_window: Res<RenderWindow>,
) {
    if let Some(surface_view) = render_window.get_view() {
        let full_output = ui_renderer.state.egui_ctx().end_pass();

        ui_renderer
            .state
            .handle_platform_output(&window.window_handle, full_output.platform_output);

        let tris = ui_renderer.state.egui_ctx().tessellate(
            full_output.shapes,
            ui_renderer.state.egui_ctx().pixels_per_point(),
        );

        for (id, image_delta) in &full_output.textures_delta.set {
            ui_renderer.renderer.update_texture(
                &render_context.device,
                &render_context.queue,
                *id,
                image_delta,
            );
        }

        let mut encoder =
            render_context
                .device
                .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                    label: Some("UI Render Encoder"),
                });

        let screen_descriptor = ScreenDescriptor {
            size_in_pixels: [
                render_context.surface_config.width,
                render_context.surface_config.height,
            ],
            pixels_per_point: window.window_handle.scale_factor() as f32,
        };

        ui_renderer.renderer.update_buffers(
            &render_context.device,
            &render_context.queue,
            &mut encoder,
            &tris,
            &screen_descriptor,
        );

        let rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &surface_view,
                resolve_target: None,
                ops: egui_wgpu::wgpu::Operations {
                    load: egui_wgpu::wgpu::LoadOp::Load,
                    store: StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            label: Some("egui main render pass"),
            occlusion_query_set: None,
        });

        ui_renderer
            .renderer
            .render(&mut rpass.forget_lifetime(), &tris, &screen_descriptor);

        for x in &full_output.textures_delta.free {
            ui_renderer.renderer.free_texture(x)
        }

        render_context.queue.submit(Some(encoder.finish()));
    }
}
