use ecs::component::Component;
use essential::assets::handle::AssetHandle;

use crate::assets::material::StandardMaterial;

#[derive(Component)]
pub struct MaterialComponent {
    pub handle: AssetHandle<StandardMaterial>,
}
