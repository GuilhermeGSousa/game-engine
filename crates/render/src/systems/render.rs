use ecs::{
    query::Query,
    resource::{Res, ResMut},
};

use crate::{
    assets::material::StandardMaterial,
    components::{
        camera::RenderCamera,
        light::RenderLights,
        mesh_component::RenderMeshInstance,
        render_material_component::RenderMaterialComponent,
        skeleton_component::{EmptySkeletonBuffer, RenderSkeletonComponent},
    },
    device::RenderDevice,
    queue::RenderQueue,
    render_asset::{
        render_material::RenderMaterial, render_mesh::RenderMesh, render_window::RenderWindow,
        RenderAssets,
    },
    resources::MainRenderPipeline,
};

pub(crate) fn main_renderpass(
    pipeline: Res<MainRenderPipeline>,
    mut device: ResMut<RenderDevice>,
    render_mesh_query: Query<(
        &RenderMeshInstance,
        Option<&RenderSkeletonComponent>,
        &RenderMaterialComponent<StandardMaterial>,
    )>,
    render_cameras: Query<&RenderCamera>,
    render_meshes: Res<RenderAssets<RenderMesh>>,
    render_materials: Res<RenderAssets<RenderMaterial>>,
    render_window: Res<RenderWindow>,
    render_lights: Res<RenderLights>,
    empty_skeleton: Res<EmptySkeletonBuffer>,
) {
    let encoder = device.command_encoder();

    for render_camera in render_cameras.iter() {
        // Route to RTT for texture-targeted cameras, or to the swapchain for window cameras.
        let swapchain_view = render_window.get_view();
        let render_view: &wgpu::TextureView = match &render_camera.render_target {
            Some(rt) => &rt.view,
            None => match swapchain_view {
                Some(v) => v,
                None => continue,
            },
        };

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Main Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: render_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(render_camera.clear_color),
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
        render_pass.set_pipeline(&pipeline);

        render_pass.set_bind_group(1, &render_camera.camera_bind_group, &[]);
        render_pass.set_bind_group(2, &render_lights.bind_group, &[]);

        for (mesh_instance, skeleton, render_mat_comp) in render_mesh_query.iter() {
            if let Some(mesh) = render_meshes.get(&mesh_instance.mesh_asset_id) {
                if let Some(render_mat) = render_materials.get(&render_mat_comp.material_asset_id) {
                    render_pass.set_bind_group(0, &render_mat.bind_group, &[]);
                } else {
                    continue;
                }

                if let Some(skeleton) = skeleton {
                    render_pass.set_bind_group(3, &skeleton.skeleton_bind_group, &[]);
                } else {
                    render_pass.set_bind_group(3, &**empty_skeleton, &[]);
                }

                render_pass.set_vertex_buffer(0, mesh.vertices.slice(..));
                render_pass.set_index_buffer(mesh.indices.slice(..), wgpu::IndexFormat::Uint32);

                render_pass.set_vertex_buffer(1, mesh_instance.transform.slice(..));
                render_pass.draw_indexed(0..mesh.index_count, 0, 0..1);
            }
        }
    }
}

pub(crate) fn present_window(mut render_window: ResMut<RenderWindow>) {
    render_window.present();
}

pub(crate) fn finish_render(mut device: ResMut<RenderDevice>, queue: Res<RenderQueue>) {
    device.finish(&queue);
}
