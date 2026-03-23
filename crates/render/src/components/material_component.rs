use ecs::component::Component;
use essential::assets::{handle::AssetHandle, Asset};

use crate::assets::material::StandardMaterial;

/// Attach this component (alongside [`MeshComponent`]) to an entity to tell the engine
/// which material the mesh should be rendered with.
///
/// The type parameter `M` defaults to [`StandardMaterial`] so existing code that writes
/// `MaterialComponent { handle: … }` with a `StandardMaterial` handle continues to work
/// without any change.  Custom materials use `MaterialComponent::<MyMaterial> { handle: … }`.
pub struct MaterialComponent<M: Asset + Send + Sync + 'static = StandardMaterial> {
    pub handle: AssetHandle<M>,
}

impl<M: Asset + Send + Sync + 'static> Component for MaterialComponent<M> {
    fn name() -> &'static str {
        std::any::type_name::<MaterialComponent<M>>()
    }
}
