use std::mem;

use ecs::{
    command::CommandQueue,
    component::Component,
    query::Query,
    resource::{Res, Resource},
};
use essential::assets::handle::AssetHandle;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BindGroupDescriptor, BindGroupEntry, BufferUsages, Device, VertexAttribute,
};

use crate::{
    assets::{texture::Texture, vertex::VertexBufferLayout},
    components::{camera::Camera, render_entity::RenderEntity},
    device::RenderDevice,
    layouts::MaterialLayouts,
    render_asset::{render_texture::RenderTexture, RenderAssets},
};

pub(crate) const SKYBOX_VERTICES: [SkyboxVertex; 8] = [
    // Front
    SkyboxVertex {
        position: [-1.0, -1.0, 1.0],
    }, // 0
    SkyboxVertex {
        position: [1.0, -1.0, 1.0],
    }, // 1
    SkyboxVertex {
        position: [1.0, 1.0, 1.0],
    }, // 2
    SkyboxVertex {
        position: [-1.0, 1.0, 1.0],
    }, // 3
    // Back
    SkyboxVertex {
        position: [-1.0, -1.0, -1.0],
    }, // 4
    SkyboxVertex {
        position: [1.0, -1.0, -1.0],
    }, // 5
    SkyboxVertex {
        position: [1.0, 1.0, -1.0],
    }, // 6
    SkyboxVertex {
        position: [-1.0, 1.0, -1.0],
    }, // 7
];

pub const SKYBOX_INDICES: [u16; 36] = [
    // Front
    0, 1, 2, 2, 3, 0, // Right
    1, 5, 6, 6, 2, 1, // Back
    5, 4, 7, 7, 6, 5, // Left
    4, 0, 3, 3, 7, 4, // Top
    3, 2, 6, 6, 7, 3, // Bottom
    4, 5, 1, 1, 0, 4,
];

#[repr(C)]
#[derive(Debug, Copy, Clone, PartialEq, bytemuck::Pod, bytemuck::Zeroable)]
pub(crate) struct SkyboxVertex {
    pub(crate) position: [f32; 3],
}

impl VertexBufferLayout for SkyboxVertex {
    fn describe() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<SkyboxVertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[VertexAttribute {
                format: wgpu::VertexFormat::Float32x3,
                offset: 0,
                shader_location: 0,
            }],
        }
    }
}

#[derive(Component)]
pub struct Skybox {
    pub texture: AssetHandle<Texture>,
}

#[derive(Component)]
pub struct RenderSkyboxBindGroup {
    pub(crate) bind_group: BindGroup,
}

#[derive(Resource)]
pub(crate) struct RenderSkyboxCube {
    pub(crate) vertices: wgpu::Buffer,
    pub(crate) indices: wgpu::Buffer,
}

impl RenderSkyboxCube {
    pub(crate) fn new(device: &Device) -> Self {
        let vertices = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("skybox_cube_vertices"),
            contents: &bytemuck::cast_slice(&SKYBOX_VERTICES),
            usage: BufferUsages::VERTEX,
        });

        let indices = device.create_buffer_init(&BufferInitDescriptor {
            label: Some("skybox_cube_indices"),
            contents: &bytemuck::cast_slice(&SKYBOX_INDICES),
            usage: BufferUsages::INDEX,
        });

        Self { vertices, indices }
    }
}

pub(crate) fn prepare_skybox(
    cameras: Query<(&Camera, &Skybox, &RenderEntity)>,
    mut cmd: CommandQueue,
    render_textures: Res<RenderAssets<RenderTexture>>,
    device: Res<RenderDevice>,
    skybox_layout: Res<MaterialLayouts>,
) {
    for (_, skybox, render_entity) in cameras.iter() {
        if let Some(render_texture) = render_textures.get(&skybox.texture.id()) {
            let bind_group = device.create_bind_group(&BindGroupDescriptor {
                    label: Some("skybox_bind_group"),
                    layout: &skybox_layout.skybox_material_layout,
                    entries: &[
                        BindGroupEntry {
                            binding: 0,
                            resource: wgpu::BindingResource::TextureView(&render_texture.view),
                        },
                        BindGroupEntry {
                            binding: 1,
                            resource: wgpu::BindingResource::Sampler(&render_texture.sampler),
                        },
                    ],
                });
                let skybox_bind_group = RenderSkyboxBindGroup { bind_group };

                cmd.insert(skybox_bind_group, **render_entity);
        }
    }
}
