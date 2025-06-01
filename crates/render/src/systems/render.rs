use ecs::{query::Query, resource::Res};
use essential::transform::Transform;
use wgpu::util::DeviceExt;

use crate::{
    mesh::{
        render_material::RenderMaterial, render_mesh::RenderMesh, render_texture::RenderTexture,
        MeshComponent,
    },
    render_asset::RenderAssets,
    resources::RenderContext,
};

pub(crate) fn render(
    context: Res<RenderContext>,
    mesh_query: Query<(&MeshComponent, &Transform)>,
    render_meshes: Res<RenderAssets<RenderMesh>>,
    render_materials: Res<RenderAssets<RenderMaterial>>,
) {
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

            render_pass.set_bind_group(1, &context.camera_bind_group, &[]);

            for (mesh, transform) in mesh_query.iter() {
                if let Some(mesh) = render_meshes.get(&mesh.handle.id()) {
                    for submesh in &mesh.sub_meshes {
                        if submesh.material.is_none() {
                            continue;
                        }
                        // TODO: Implement default material
                        // let material_asset = match submesh.material
                        // {
                        //     Some(material) => material,
                        //     None => todo!(),
                        // }

                        if let Some(render_mat) = render_materials.get(&submesh.material.unwrap()) {
                            render_pass.set_bind_group(0, &render_mat.diffuse_bind_group, &[]);
                        } else {
                            continue;
                        }

                        render_pass.set_vertex_buffer(0, submesh.vertices.slice(..));
                        render_pass
                            .set_index_buffer(submesh.indices.slice(..), wgpu::IndexFormat::Uint32);

                        let instance_buffer =
                            context
                                .device
                                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                                    label: Some("Instance Buffer"),
                                    contents: bytemuck::cast_slice(&[transform.to_raw()]),
                                    usage: wgpu::BufferUsages::VERTEX,
                                });
                        render_pass.set_vertex_buffer(1, instance_buffer.slice(..));

                        render_pass.draw_indexed(0..submesh.index_count, 0, 0..1);
                    }
                }
            }
        }
        context.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }
}
