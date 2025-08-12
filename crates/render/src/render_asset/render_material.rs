use ecs::resource::Res;
use wgpu::{util::{BufferInitDescriptor, DeviceExt}, BufferUsages};

use crate::{
    assets::material::{Material, MaterialFlags},
    layouts::MeshLayouts,
    render_asset::{
        render_texture::DummyRenderTexture, AssetPreparationError, RenderAsset, RenderAssets,
    },
    resources::RenderContext,
};

use super::render_texture::RenderTexture;

pub(crate) struct RenderMaterial {
    pub(crate) bind_group: wgpu::BindGroup,
}

impl RenderAsset for RenderMaterial {
    type SourceAsset = Material;

    type PreparationParams = (
        Res<'static, RenderContext>,
        Res<'static, RenderAssets<RenderTexture>>,
        Res<'static, DummyRenderTexture>,
        Res<'static, MeshLayouts>,
    );

    fn prepare_asset(
        source_asset: &Self::SourceAsset,
        params: &mut ecs::system::system_input::SystemInputData<Self::PreparationParams>,
    ) -> Result<Self, AssetPreparationError> {
        let (render_context, render_textures, dummy_texture, mesh_layouts) = params;

        let mut entries = Vec::new();

        if let Some(diffuse_tex_handle) = source_asset.diffuse_texture() {
            if let Some(diffuse_tex) = render_textures.get(&diffuse_tex_handle.id()) {
                entries.push(wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&diffuse_tex.view),
                });
                entries.push(wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&diffuse_tex.sampler),
                });
            } else {
                return Err(AssetPreparationError::NotReady);
            }
        } else {
            entries.push(wgpu::BindGroupEntry {
                binding: 0,
                resource: wgpu::BindingResource::TextureView(&dummy_texture.view),
            });
            entries.push(wgpu::BindGroupEntry {
                binding: 1,
                resource: wgpu::BindingResource::Sampler(&dummy_texture.sampler),
            });
        }

        if let Some(normal_tex_handle) = source_asset.normal_texture() {
            if let Some(normal_tex) = render_textures.get(&normal_tex_handle.id()) {
                entries.push(wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::TextureView(&normal_tex.view),
                });
                entries.push(wgpu::BindGroupEntry {
                    binding: 3,
                    resource: wgpu::BindingResource::Sampler(&normal_tex.sampler),
                });
            } else {
                return Err(AssetPreparationError::NotReady);
            }
        } else {
            entries.push(wgpu::BindGroupEntry {
                binding: 2,
                resource: wgpu::BindingResource::TextureView(&dummy_texture.view),
            });
            entries.push(wgpu::BindGroupEntry {
                binding: 3,
                resource: wgpu::BindingResource::Sampler(&dummy_texture.sampler),
            });
        }

        let material_flags_buffer = render_context.device.create_buffer_init(
            &BufferInitDescriptor {
                label: Some("material_flags"),
                contents: bytemuck::cast_slice(&[MaterialFlags::from_material(source_asset)]),
                usage: BufferUsages::UNIFORM,
            }
            );
        
        entries.push(wgpu::BindGroupEntry {
            binding: 4,
            resource: wgpu::BindingResource::Buffer(material_flags_buffer.as_entire_buffer_binding())
        });

        let bind_group = render_context
            .device
            .create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &mesh_layouts.mesh_layout,
                entries: &entries,
                label: Some("material_bind_group"),
            });

        Ok(RenderMaterial { bind_group })
    }
}

impl RenderMaterial {}
