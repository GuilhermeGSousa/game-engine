use ecs::resource::Res;
use wgpu::util::DeviceExt;

use crate::{
    assets::mesh::Mesh,
    device::RenderDevice,
    render_asset::{AssetPreparationError, RenderAsset},
};

pub(crate) struct RenderPrimitive {
    pub(crate) vertices: wgpu::Buffer,
    pub(crate) indices: wgpu::Buffer,
    pub(crate) index_count: u32,
    pub(crate) material: Option<AssetId>,
}

pub(crate) struct RenderMesh {
    pub(crate) sub_meshes: Vec<RenderPrimitive>,
}

impl RenderAsset for RenderMesh {
    type SourceAsset = Mesh;

    type PreparationParams = (Res<'static, RenderDevice>,);

    fn prepare_asset(
        source_asset: &Self::SourceAsset,
        params: &mut ecs::system::system_input::SystemInputData<Self::PreparationParams>,
    ) -> Result<Self, AssetPreparationError> {
        let (context,) = params;

        let sub_meshes = source_asset
            .primitives
            .iter()
            .map(|sub_mesh| {
                let vertices =
                    context
                        .device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Vertex Buffer"),
                            contents: bytemuck::cast_slice(&sub_mesh.vertices),
                            usage: wgpu::BufferUsages::VERTEX,
                        });
                let indices =
                    context
                        .device
                        .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                            label: Some("Index Buffer"),
                            contents: bytemuck::cast_slice(&sub_mesh.indices),
                            usage: wgpu::BufferUsages::INDEX,
                        });
                let index_count = sub_mesh.indices.len() as u32;
                let material = source_asset
                    .materials
                    .get(sub_mesh.material_index)
                    .map(|m| m.id());

                RenderPrimitive {
                    vertices,
                    indices,
                    index_count,
                    material,
                }
            })
            .collect();

        Ok(RenderMesh { sub_meshes })
    }
}
