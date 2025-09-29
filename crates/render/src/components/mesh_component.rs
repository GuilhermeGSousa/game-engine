use ecs::{
    command::CommandQueue,
    component::Component,
    query::{
        query_filter::{Added, Changed},
        Query,
    },
    resource::Res,
};
use essential::{
    assets::{handle::AssetHandle, AssetId},
    transform::{GlobalTranform, Transform},
};
use wgpu::util::DeviceExt;

use crate::{
    assets::mesh::Mesh,
    components::{material_component::MaterialComponent, render_entity::RenderEntity},
    device::RenderDevice,
    queue::RenderQueue,
};

#[derive(Component)]
pub struct MeshComponent {
    pub handle: AssetHandle<Mesh>,
}

#[derive(Component)]
pub(crate) struct RenderMeshInstance {
    pub(crate) mesh_asset_id: AssetId,
    pub(crate) material_asset_id: AssetId,
    pub(crate) buffer: wgpu::Buffer,
}

pub(crate) fn mesh_added(
    meshes: Query<
        (
            &MeshComponent,
            &MaterialComponent,
            &GlobalTranform,
            &mut RenderEntity,
        ),
        Added<(MeshComponent,)>,
    >,
    mut cmd: CommandQueue,
    device: Res<RenderDevice>,
) {
    for (mesh, material, transform, mut render_entity) in meshes.iter() {
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&[transform.to_raw()]),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let instance: RenderMeshInstance = RenderMeshInstance {
            mesh_asset_id: mesh.handle.id(),
            material_asset_id: material.handle.id(),
            buffer: instance_buffer,
        };

        match **render_entity {
            RenderEntity::Uninitialized => {
                let new_entity = cmd.spawn(instance);
                render_entity.set_entity(new_entity);
            }
            RenderEntity::Initialized(entity) => {
                cmd.insert(instance, entity);
            }
        }
    }
}

pub(crate) fn mesh_changed(
    meshes: Query<(&MeshComponent, &GlobalTranform, &RenderEntity), Changed<(Transform,)>>,
    render_meshes: Query<(&mut RenderMeshInstance,)>,
    queue: Res<RenderQueue>,
) {
    for (_, transform, render_entity) in meshes.iter() {
        match render_entity {
            RenderEntity::Initialized(entity) => {
                if let Some((render_mesh,)) = render_meshes.get_entity(*entity) {
                    queue.write_buffer(
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
