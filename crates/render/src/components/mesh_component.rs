use ecs::component::Component;
use essential::assets::{handle::AssetHandle, AssetId};

use crate::assets::mesh::Mesh;

#[derive(Component)]
pub struct MeshComponent {
    pub handle: AssetHandle<Mesh>,
}

#[derive(Component)]
pub(crate) struct RenderMeshInstance {
    pub(crate) render_asset_id: AssetId,
    pub(crate) buffer: wgpu::Buffer,
}
