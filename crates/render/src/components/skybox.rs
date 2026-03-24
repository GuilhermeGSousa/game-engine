use std::mem;

use ecs::{
    command::CommandQueue,
    component::Component,
    query::{query_filter::With, Query},
    resource::{Res, Resource},
};
use essential::assets::handle::AssetHandle;
use wgpu::{
    util::{BufferInitDescriptor, DeviceExt},
    BindGroup, BufferUsages, Device, VertexAttribute,
};

use crate::{
    assets::{
        material::AsBindGroup,
        skybox_material::SkyboxMaterial,
        texture::Texture,
        vertex::VertexBufferLayout,
    },
    components::{
        camera::{Camera, RenderCamera},
        render_entity::RenderEntity,
    },
    device::RenderDevice,
    render_asset::{
        render_texture::{DummyRenderTexture, RenderTexture},
        RenderAssets,
    },
    resources::SkyboxRenderPipeline,
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

/// Creates [`RenderSkyboxBindGroup`] on render-camera entities once the
/// skybox texture asset has been loaded into the render world.
///
/// The bind group is built via [`SkyboxMaterial::create_bind_group`] so the
/// layout is derived from the same `#[derive(AsBindGroup)]` macro that
/// produced the pipeline's `@group(0)` layout.
pub(crate) fn prepare_skybox(
    cameras: Query<(&Skybox, &RenderEntity), With<Camera>>,
    render_cameras: Query<(&RenderCamera, &RenderSkyboxBindGroup)>,
    mut cmd: CommandQueue,
    render_textures: Res<RenderAssets<RenderTexture>>,
    device: Res<RenderDevice>,
    skybox_pipeline: Res<SkyboxRenderPipeline>,
    dummy_texture: Res<DummyRenderTexture>,
) {
    for (skybox, render_entity) in cameras.iter() {
        if render_cameras.contains_entity(**render_entity) {
            continue;
        }

        let skybox_mat = SkyboxMaterial {
            texture: Some(skybox.texture.clone()),
        };

        if let Ok(bind_group) = skybox_mat.create_bind_group(
            &device,
            &render_textures,
            &dummy_texture,
            &skybox_pipeline.material_layout,
        ) {
            cmd.insert(RenderSkyboxBindGroup { bind_group }, **render_entity);
        }
    }
}
