use ecs::resource::Res;

use crate::resources::RenderContext;

pub(crate) fn render(context: Res<RenderContext>) {
    if let Ok(output) = context.surface.get_current_texture() {
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = context
            .device
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
            render_pass.set_pipeline(&context.pipeline);

            render_pass.set_bind_group(0, &context.camera_bind_group, &[]);

            // render_state.meshes.iter().for_each(|mesh| {
            //     render_pass.set_bind_group(1, &mesh.material.diffuse_bind_group, &[]);
            //     render_pass.set_vertex_buffer(0, mesh.vertices.slice(..));
            //     render_pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
            //     render_pass.set_vertex_buffer(1, mesh.instance_buffer.slice(..));
            //     render_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            // });
        }
        context.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }
}
