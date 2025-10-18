use ecs::resource::Res;

use crate::{assets::skeleton::Skeleton, device::RenderDevice, render_asset::RenderAsset};

pub(crate) struct RenderSkeleton {
    pub(crate) inverse_bindposes: wgpu::Buffer,
}

impl RenderAsset for RenderSkeleton 
{
    type SourceAsset = Skeleton;

    type PreparationParams = (Res<'static, RenderDevice>,);

    fn prepare_asset(
        source_asset: &Self::SourceAsset,
        params: &mut ecs::system::system_input::SystemInputData<Self::PreparationParams>,
    ) -> Result<Self, super::AssetPreparationError> {
        todo!()
    }
}