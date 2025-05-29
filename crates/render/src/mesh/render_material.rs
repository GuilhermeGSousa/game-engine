use ecs::resource::Res;

use crate::{
    render_asset::{RenderAsset, RenderAssets},
    resources::RenderContext,
};

use super::{layouts::MeshLayouts, material::Material, render_texture::RenderTexture};

pub(crate) struct RenderMaterial {
    pub(crate) diffuse_bind_group: wgpu::BindGroup,
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
    ) -> Self {
        let (render_context, render_textures, mesh_layouts) = params;

        let diffuse_texture = source_asset.diffuse_texture();

        let mut entries = Vec::new();

        if let Some(diffuse_texture) = diffuse_texture {
            if let Some(render_texture) = render_textures.get(&diffuse_texture.id()) {
                entries.push(wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(&render_texture.view),
                });
                entries.push(wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&render_texture.sampler),
                });
            }
        }

        let diffuse_bind_group =
            render_context
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    layout: &mesh_layouts.mesh_layout,
                    entries: &entries,
                    label: Some("diffuse_bind_group"),
                });

        RenderMaterial { diffuse_bind_group }
    }
}
