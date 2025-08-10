use ecs::resource::Res;

use crate::{
    assets::material::Material,
    layouts::MeshLayouts,
    render_asset::{AssetPreparationError, RenderAsset, RenderAssets},
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
        Res<'static, MeshLayouts>,
    );

    fn prepare_asset(
        source_asset: &Self::SourceAsset,
        params: &mut ecs::system::system_input::SystemInputData<Self::PreparationParams>,
    ) -> Result<Self, AssetPreparationError> {
        let (render_context, render_textures, mesh_layouts) = params;

        match (
            source_asset.diffuse_texture(),
            source_asset.normal_texture(),
        ) {
            (Some(diffuse_tex_handle), Some(normal_tex_handle)) => {
                match (
                    render_textures.get(&diffuse_tex_handle.id()),
                    render_textures.get(&normal_tex_handle.id()),
                ) {
                    (Some(diffuse_tex), Some(normal_tex)) => {
                        let bind_group =
                            render_context
                                .device
                                .create_bind_group(&wgpu::BindGroupDescriptor {
                                    layout: &mesh_layouts.mesh_layout,
                                    entries: &[
                                        wgpu::BindGroupEntry {
                                            binding: 0,
                                            resource: wgpu::BindingResource::TextureView(
                                                &diffuse_tex.view,
                                            ),
                                        },
                                        wgpu::BindGroupEntry {
                                            binding: 1,
                                            resource: wgpu::BindingResource::Sampler(
                                                &diffuse_tex.sampler,
                                            ),
                                        },
                                        wgpu::BindGroupEntry {
                                            binding: 2,
                                            resource: wgpu::BindingResource::TextureView(
                                                &normal_tex.view,
                                            ),
                                        },
                                        wgpu::BindGroupEntry {
                                            binding: 3,
                                            resource: wgpu::BindingResource::Sampler(
                                                &normal_tex.sampler,
                                            ),
                                        },
                                    ],
                                    label: Some("material_bind_group"),
                                });
                        Ok(RenderMaterial { bind_group })
                    }
                    _ => Err(AssetPreparationError::NotReady),
                }
            }
            _ => Err(AssetPreparationError::NotReady),
        }
    }
}
