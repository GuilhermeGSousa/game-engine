use ecs::component::Component;
use essential::assets::handle::AssetHandle;

use crate::assets::material::Material;

#[derive(Component)]
pub struct MaterialComponent {
    pub handle: AssetHandle<Material>,
}
