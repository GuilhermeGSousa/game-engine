use ecs::{
    query::Query,
    resource::{Res, ResMut},
};

use crate::{
    components::{camera::RenderCamera, light::RenderLights, mesh_component::RenderMeshInstance},
    device::RenderDevice,
    queue::RenderQueue,
    render_asset::{
        render_material::RenderMaterial, render_mesh::RenderMesh, render_window::RenderWindow,
        RenderAssets,
    },
    resources::RenderContext,
};

pub(crate) fn render(
    context: Res<RenderContext>,
    device: Res<RenderDevice>,
    queue: Res<RenderQueue>,
    render_mesh_query: Query<(&RenderMeshInstance,)>,
    render_cameras: Query<&RenderCamera>,
    render_meshes: Res<RenderAssets<RenderMesh>>,
    render_window: Res<RenderWindow>,
    render_materials: Res<RenderAssets<RenderMaterial>>,
    render_lights: Res<RenderLights>,
) {
    if let Some(view) = render_window.get_view() {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            for render_camera in render_cameras.iter() {
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
                    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                        view: &render_camera.depth_texture.view,
                        depth_ops: Some(wgpu::Operations {
                            load: wgpu::LoadOp::Clear(1.0),
                            store: wgpu::StoreOp::Store,
                        }),
                        stencil_ops: None,
                    }),
                    occlusion_query_set: None,
                    timestamp_writes: None,
                });
                render_pass.set_pipeline(&context.main_pipeline);

                render_pass.set_bind_group(1, &render_camera.camera_bind_group, &[]);
                render_pass.set_bind_group(2, &render_lights.bind_group, &[]);

                for (mesh_instance,) in render_mesh_query.iter() {
                    if let Some(mesh) = render_meshes.get(&mesh_instance.render_asset_id) {
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

                            if let Some(render_mat) =
                                render_materials.get(&submesh.material.unwrap())
                            {
                                render_pass.set_bind_group(0, &render_mat.bind_group, &[]);
                            } else {
                                continue;
                            }

                            render_pass.set_vertex_buffer(0, submesh.vertices.slice(..));
                            render_pass.set_index_buffer(
                                submesh.indices.slice(..),
                                wgpu::IndexFormat::Uint32,
                            );

                            render_pass.set_vertex_buffer(1, mesh_instance.buffer.slice(..));
                            render_pass.draw_indexed(0..submesh.index_count, 0, 0..1);
                        }
                    }
                }
            }
        }
        queue.submit(std::iter::once(encoder.finish()));
    }
}

pub(crate) fn present_window(mut render_window: ResMut<RenderWindow>) {
    render_window.present();
}
