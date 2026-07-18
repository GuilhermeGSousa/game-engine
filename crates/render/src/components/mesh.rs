use ecs::{
    component::Component,
    query::{query_filter::Changed, Query},
    resource::Res,
    Added, CommandQueue, Entity,
};
use essential::{
    assets::AssetId,
    transform::{GlobalTransform, GlobalTransformRaw},
};
use glam::Mat4;
use mesh::{mesh::MeshComponent, skeleton::SkeletonComponent};
use wgpu::util::DeviceExt;

use crate::{components::render_entity::RenderEntity, device::RenderDevice, queue::RenderQueue};

#[derive(Component)]
pub(crate) struct RenderMeshInstance {
    pub(crate) mesh_asset_id: AssetId,
    pub(crate) transform: wgpu::Buffer,
    // CPU-side cache of `transform`'s contents, kept in sync alongside it so that
    // `sync_instance_membership` (instance_batch.rs) can read a live entity's current
    // matrix without a GPU buffer readback.
    pub(crate) transform_raw: GlobalTransformRaw,
}

// The instance transform written to the GPU. Non-skinned meshes render at their
// propagated world transform; skinned meshes must stay identity because their bone
// palette already carries the world transform (see `update_skeletons`), so applying
// the world transform here too would double-transform them.
fn instance_transform_raw(transform: &GlobalTransform, skinned: bool) -> GlobalTransformRaw {
    if skinned {
        GlobalTransform::new(Mat4::IDENTITY).to_raw()
    } else {
        transform.to_raw()
    }
}

pub(crate) fn mesh_added(
    meshes: Query<
        (
            Entity,
            &MeshComponent,
            &GlobalTransform,
            Option<&SkeletonComponent>,
            Option<&RenderEntity>,
        ),
        Added<(MeshComponent,)>,
    >,
    mut cmd: CommandQueue,
    device: Res<RenderDevice>,
) {
    for (entity, mesh, transform, skeleton, render_entity) in meshes.iter() {
        let transform_raw = instance_transform_raw(transform, skeleton.is_some());
        let instance_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Instance Buffer"),
            contents: bytemuck::cast_slice(&[transform_raw]),
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
        });

        let instance = RenderMeshInstance {
            mesh_asset_id: mesh.handle.id(),
            transform: instance_buffer,
            transform_raw,
        };

        match render_entity {
            Some(render_entity) => {
                cmd.insert(instance, **render_entity);
            }
            None => {
                let new_render_entity = cmd.spawn(instance).entity();
                cmd.insert(RenderEntity::new(new_render_entity), entity);
            }
        }
    }
}

pub(crate) fn mesh_changed(
    meshes: Query<
        (
            &MeshComponent,
            &GlobalTransform,
            Option<&SkeletonComponent>,
            &RenderEntity,
        ),
        Changed<(GlobalTransform,)>,
    >,
    render_meshes: Query<(&mut RenderMeshInstance,)>,
    queue: Res<RenderQueue>,
) {
    for (_, transform, skeleton, render_entity) in meshes.iter() {
        if let Some((mut render_mesh,)) = render_meshes.get_entity(**render_entity) {
            let transform_raw = instance_transform_raw(transform, skeleton.is_some());
            queue.write_buffer(
                &render_mesh.transform,
                0,
                bytemuck::cast_slice(&[transform_raw]),
            );
            render_mesh.transform_raw = transform_raw;
        }
    }
}
