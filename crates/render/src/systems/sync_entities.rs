use ecs::{
    command::CommandQueue,
    query::Query,
    query_filter::{Added, Changed},
    resource::Res,
};
use essential::transform::Transform;
use wgpu::util::DeviceExt;

use crate::{
    components::{
        camera::{Camera, CameraUniform, RenderCamera},
        mesh_component::MeshComponent,
        render_entity::RenderEntity,
    },
    layouts::CameraLayouts,
    render_asset::{
        render_mesh::{self, RenderMeshInstance},
        render_texture::RenderTexture,
    },
    resources::RenderContext,
};

pub(crate) fn camera_added(
    cameras: Query<(&Camera, &Transform, &mut RenderEntity), Added<(Camera,)>>,
    mut cmd: CommandQueue,
    context: Res<RenderContext>,
    camera_layouts: Res<CameraLayouts>,
) {
    for (camera, transform, render_entity_component) in cameras.iter() {
        let mut camera_uniform = CameraUniform::new();
        camera_uniform.update_view_proj(camera, transform);

        let camera_buffer = context
            .device
            .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            });

        let camera_bind_group = context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &camera_layouts.camera_layout,
                entries: &[wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }],
                label: Some("camera_bind_group"),
            });

        let depth_texture = RenderTexture::create_depth_texture(
            &context.device,
            &context.surface_config,
            "depth_texture",
        );

        context
            .queue
            .write_buffer(&camera_buffer, 0, bytemuck::cast_slice(&[camera_uniform]));

        let render_entity = cmd.spawn((RenderCamera {
            camera_bind_group: camera_bind_group,
            camera_uniform: camera_uniform,
            camera_buffer: camera_buffer,
            depth_texture: depth_texture,
        },));

        render_entity_component.set_entity(render_entity);
    }
}

pub(crate) fn camera_moved(
    cameras: Query<(&Camera, &Transform, &RenderEntity), Changed<(Transform,)>>,
    render_cameras: Query<(&mut RenderCamera,)>,
    context: Res<RenderContext>,
) {
    for (camera, transform, render_entity) in cameras.iter() {
        match render_entity {
            RenderEntity::Initialized(entity) => {
                if let Some((render_camera,)) = render_cameras.get_entity(*entity) {
                    render_camera
                        .camera_uniform
                        .update_view_proj(camera, transform);

                    context.queue.write_buffer(
                        &render_camera.camera_buffer,
                        0,
                        bytemuck::cast_slice(&[render_camera.camera_uniform]),
                    );
                }
            }
            _ => {}
        }
    }
}

pub(crate) fn mesh_added(
    meshes: Query<(&MeshComponent, &Transform, &mut RenderEntity), Added<(MeshComponent,)>>,
    mut cmd: CommandQueue,
    context: Res<RenderContext>,
) {
    for (mesh, transform, render_entity) in meshes.iter() {
        let instance_buffer =
            context
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("Instance Buffer"),
                    contents: bytemuck::cast_slice(&[transform.to_raw()]),
                    usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                });

        let instance = RenderMeshInstance {
            render_asset_id: mesh.handle.id(),
            buffer: instance_buffer,
        };

        let mesh_instance_entity = cmd.spawn((instance,));
        render_entity.set_entity(mesh_instance_entity);
    }
}

pub(crate) fn mesh_moved(
    meshes: Query<(&MeshComponent, &Transform, &RenderEntity), Changed<(Transform,)>>,
    render_meshes: Query<(&mut RenderMeshInstance,)>,
    context: Res<RenderContext>,
) {
    for (_, transform, render_entity) in meshes.iter() {
        match render_entity {
            RenderEntity::Initialized(entity) => {
                if let Some((render_mesh,)) = render_meshes.get_entity(*entity) {
                    context.queue.write_buffer(
                        &render_mesh.buffer,
                        0,
                        bytemuck::cast_slice(&[transform.to_raw()]),
                    );
                }
            }
            _ => {}
        }
    }
}
