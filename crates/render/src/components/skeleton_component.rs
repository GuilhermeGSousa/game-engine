use std::ops::Deref;

use crate::{
    assets::skeleton::Skeleton, components::render_entity::RenderEntity, device::RenderDevice,
    layouts::SkeletonLayout, queue::RenderQueue,
};
use ecs::{
    command::CommandQueue,
    component::Component,
    entity::Entity,
    query::{query_filter::Added, Query},
    resource::{Res, Resource},
};
use encase::{ShaderType, UniformBuffer};
use essential::{
    assets::{asset_store::AssetStore, handle::AssetHandle},
    transform::GlobalTranform,
};
use glam::Mat4;
use wgpu::{util::DeviceExt, BindGroupDescriptor, BufferDescriptor, Device};

const MAX_SKELETON_BONES: usize = 256;
const BONE_SIZE: usize = size_of::<Mat4>();

#[derive(Component)]
pub struct SkeletonComponent {
    skeleton: AssetHandle<Skeleton>,
    bones: Vec<Entity>,
}

impl SkeletonComponent {
    pub fn new(skeleton: AssetHandle<Skeleton>, bones: Vec<Entity>) -> Self {
        Self { skeleton, bones }
    }
}
#[derive(Component)]
pub struct RenderSkeletonComponent {
    pub(crate) bones: wgpu::Buffer,
    pub(crate) skeleton_bind_group: wgpu::BindGroup,
}

#[derive(Resource)]
pub(crate) struct EmptySkeletonBuffer(wgpu::BindGroup);

impl EmptySkeletonBuffer {
    pub(crate) fn new(device: &Device, layout: &SkeletonLayout) -> Self {
        let mut buffer = UniformBuffer::new(Vec::new());
        buffer.write(&[Mat4::IDENTITY; MAX_SKELETON_BONES]).unwrap();

        let skeleton_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("lights_buffer"),
            contents: &buffer.into_inner(),
            usage: wgpu::BufferUsages::UNIFORM,
        });

        Self(device.create_bind_group(&BindGroupDescriptor {
            label: Some("empty_skeleton_buffer"),
            layout: *&layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: skeleton_buffer.as_entire_binding(),
            }],
        }))
    }
}

impl Deref for EmptySkeletonBuffer {
    type Target = wgpu::BindGroup;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub(crate) fn skeleton_added(
    skeletons: Query<(Entity, Option<&RenderEntity>), Added<(SkeletonComponent,)>>,
    skeleton_layout: Res<SkeletonLayout>,
    mut cmd: CommandQueue,
    device: Res<RenderDevice>,
) {
    for (entity, render_entity) in skeletons.iter() {
        let bones_buffer = device.create_buffer(&BufferDescriptor {
            label: Some("Skeleton Buffer"),
            size: (MAX_SKELETON_BONES * BONE_SIZE) as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let skeleton_bind_group = device.create_bind_group(&BindGroupDescriptor {
            label: Some("Skeleton Bind Group"),
            layout: &*skeleton_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: bones_buffer.as_entire_binding(),
            }],
        });

        let render_skeleton_component = RenderSkeletonComponent {
            bones: bones_buffer,
            skeleton_bind_group,
        };

        match render_entity {
            Some(render_entity) => {
                cmd.insert(render_skeleton_component, **render_entity);
            }
            None => {
                let new_render_entity = cmd.spawn(render_skeleton_component);
                cmd.insert(RenderEntity::new(new_render_entity), entity);
            }
        }
    }
}

pub(crate) fn update_skeletons(
    skeletons: Query<(&SkeletonComponent, &RenderEntity)>,
    render_skeletons: Query<&RenderSkeletonComponent>,
    transforms: Query<&GlobalTranform>,
    skeleton_assets: Res<AssetStore<Skeleton>>,
    queue: Res<RenderQueue>,
) {
    for (skeleton, render_entity) in skeletons.iter() {
        let render_skeleton = render_skeletons.get_entity(**render_entity);

        match (skeleton_assets.get(&skeleton.skeleton), render_skeleton) {
            (Some(skeleton_asset), Some(render_skeleton)) => {
                let mut bone_transforms = [Mat4::IDENTITY; MAX_SKELETON_BONES];

                for (bone_index, (inverse_bindpose, bone_entity)) in skeleton_asset
                    .inverse_bindposes
                    .iter()
                    .zip(&skeleton.bones)
                    .enumerate()
                {
                    let transform = match transforms.get_entity(*bone_entity) {
                        Some(bone_transform) => bone_transform.matrix() * *inverse_bindpose,
                        None => Mat4::IDENTITY,
                    };

                    bone_transforms[bone_index] = transform;
                }

                let mut buffer = UniformBuffer::new(Vec::new());
                buffer.write(&bone_transforms).unwrap();
                queue.write_buffer(&render_skeleton.bones, 0, &buffer.into_inner());
            }
            _ => continue,
        };
    }
}

#[derive(ShaderType)]
pub(crate) struct SkeletonUniform {
    pub(crate) bones: [Mat4; MAX_SKELETON_BONES],
}
