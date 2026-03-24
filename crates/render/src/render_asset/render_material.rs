use ecs::resource::Res;

use crate::{
    assets::material::{AsBindGroup, StandardMaterial},
    device::RenderDevice,
    layouts::MaterialLayouts,
    render_asset::{
        render_texture::DummyRenderTexture, AssetPreparationError, RenderAsset, RenderAssets,
    },
};

use super::render_texture::RenderTexture;

pub(crate) struct RenderMaterial {
    pub(crate) bind_group: wgpu::BindGroup,
}

impl RenderAsset for RenderMaterial {
    type SourceAsset = StandardMaterial;

    type PreparationParams = (
        Res<'static, RenderDevice>,
        Res<'static, RenderAssets<RenderTexture>>,
        Res<'static, DummyRenderTexture>,
        Res<'static, MaterialLayouts>,
    );

    fn prepare_asset(
        source_asset: &Self::SourceAsset,
        params: &mut ecs::system::system_input::SystemInputData<Self::PreparationParams>,
    ) -> Result<Self, AssetPreparationError> {
        let (device, render_textures, dummy_texture, mesh_layouts) = params;

        let bind_group = source_asset.create_bind_group(
            device,
            render_textures,
            dummy_texture,
            &mesh_layouts.main_material_layout,
        )?;

        Ok(RenderMaterial { bind_group })
    }
}
