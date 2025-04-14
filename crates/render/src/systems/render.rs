use essential::transform::Transform;

use ecs::{
    query::Query,
    resource::{Res, ResMut},
};
use wgpu::util::DeviceExt;

use crate::{
    mesh::{render_mesh::RenderMesh, MeshComponent},
    resources::{RenderContext, RenderWorldState},
};

pub(crate) fn prepare_render_state(
    meshes: Query<(&MeshComponent, &Transform)>,
    context: Res<RenderContext>,
    mut render_state: ResMut<RenderWorldState>,
) {
    render_state.clear();

    for (mesh, transform) in meshes.iter() {
        let mesh = RenderMesh {
            vertices: context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Vertex Buffer"),
                    contents: bytemuck::cast_slice(&mesh.mesh_asset.vertices),
                    usage: wgpu::BufferUsages::VERTEX,
                }),
            indices: context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Index Buffer"),
                    contents: bytemuck::cast_slice(&mesh.mesh_asset.indices),
                    usage: wgpu::BufferUsages::INDEX,
                }),
            index_count: mesh.mesh_asset.indices.len() as u32,
            instance_buffer: context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Instance Buffer"),
                    contents: bytemuck::cast_slice(&[transform.to_raw()]),
                    usage: wgpu::BufferUsages::VERTEX,
                }),
        };
        render_state.add_mesh(mesh);
    }
}

pub(crate) fn render(render_state: Res<RenderWorldState>, context: Res<RenderContext>) {
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

            render_pass.set_bind_group(0, &context.diffuse_bind_group, &[]);
            render_pass.set_bind_group(1, &context.camera_bind_group, &[]);

            render_state.meshes.iter().for_each(|mesh| {
                render_pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                render_pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);
                render_pass.set_vertex_buffer(1, mesh.instance_buffer.slice(..));
                render_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            });
        }
        context.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }
}
