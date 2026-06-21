use ecs::{
    component::Component,
    query::{query_filter::Changed, Query},
    resource::Res,
    Added, CommandQueue, Entity,
};
use essential::{
    assets::AssetId,
    transform::{GlobalTransform, Transform},
};
use mesh::mesh::MeshComponent;
use wgpu::util::DeviceExt;

use crate::{components::render_entity::RenderEntity, device::RenderDevice, queue::RenderQueue};

#[derive(Component)]
pub(crate) struct RenderMeshInstance {
    pub(crate) mesh_asset_id: AssetId,
    pub(crate) transform: wgpu::Buffer,
}

pub(crate) fn mesh_added(
    meshes: Query<
        (
            Entity,
            &MeshComponent,
            &GlobalTransform,
            Option<&RenderEntity>,
        ),
        Added<(MeshComponent,)>,
    >,
    mut cmd: CommandQueue,
    device: Res<RenderDevice>,
) {
    for (entity, mesh, transform, render_entity) in meshes.iter() {
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&[transform.to_raw()]),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let instance = RenderMeshInstance {
            mesh_asset_id: mesh.handle.id(),
            transform: instance_buffer,
        };

        match render_entity {
            Some(render_entity) => {
                cmd.insert(instance, **render_entity);
            }
            None => {
                let new_render_entity = *cmd.spawn(instance).entity();
                cmd.insert(RenderEntity::new(new_render_entity), entity);
            }
        }
    }
}

pub(crate) fn mesh_changed(
    meshes: Query<(&MeshComponent, &GlobalTransform, &RenderEntity), Changed<(Transform,)>>,
    render_meshes: Query<(&mut RenderMeshInstance,)>,
    queue: Res<RenderQueue>,
) {
    for (_, transform, render_entity) in meshes.iter() {
        if let Some((render_mesh,)) = render_meshes.get_entity(**render_entity) {
            queue.write_buffer(
                &render_mesh.transform,
                0,
                bytemuck::cast_slice(&[transform.to_raw()]),
            );
        }
    }
}
