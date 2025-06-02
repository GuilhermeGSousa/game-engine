use ecs::component::Component;
use essential::assets::handle::AssetHandle;

use crate::assets::mesh::Mesh;

#[derive(Component)]
pub struct MeshComponent {
    pub handle: AssetHandle<Mesh>,
}
