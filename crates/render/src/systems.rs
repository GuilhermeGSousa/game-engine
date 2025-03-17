use core::resource::{Res, ResMut};

use window::plugin::Window;

use crate::{
    resources::{
        RenderBuffer, RenderConfig, RenderDevice, RenderDifuseBindGroup, RenderPipeline,
        RenderQueue, RenderSurface,
    },
    vertex::{INDICES, VERTICES},
};

pub(crate) fn render(
    mut window: ResMut<Window>,
    surface: Res<RenderSurface>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    mut config: ResMut<RenderConfig>,
    pipeline: Res<RenderPipeline>,
    buffer: Res<RenderBuffer>,
    difuse_bind_group: Res<RenderDifuseBindGroup>,
) {
    if window.should_resize() {
        let size = window.size();
        let surface = surface.0.clone();
        config.0.width = size.0;
        config.0.height = size.1;
        surface.configure(&device.0, &config.0);
        window.clear_resize();
    }

    if let Ok(output) = surface.0.get_current_texture() {
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = device
            .0
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.1,
                            g: 0.2,
                            b: 0.3,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });
            render_pass.set_pipeline(&pipeline.0);
            render_pass.set_bind_group(0, &difuse_bind_group.0, &[]);
            render_pass.set_vertex_buffer(0, buffer.vertex_buffer.slice(..));

            let num_indices = INDICES.len() as u32;
            render_pass.set_index_buffer(buffer.index_buffer.slice(..), wgpu::IndexFormat::Uint16); // 1.
            render_pass.draw_indexed(0..num_indices, 0, 0..1); // 2.
        }

        // UI stuff must run here

        queue.0.submit(std::iter::once(encoder.finish()));
        output.present();
    }
}
