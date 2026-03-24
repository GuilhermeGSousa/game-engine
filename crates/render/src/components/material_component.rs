use ecs::component::Component;
use essential::assets::handle::AssetHandle;

use crate::{assets::material::StandardMaterial, Material};

/// Attach this component (alongside [`MeshComponent`]) to an entity to tell the engine
/// which material the mesh should be rendered with.
///
/// The type parameter `M` defaults to [`StandardMaterial`] so existing code that writes
/// `MaterialComponent { handle: … }` with a `StandardMaterial` handle continues to work
/// without any change.  Custom materials use `MaterialComponent::<MyMaterial> { handle: … }`.
#[derive(Component)]
pub struct MaterialComponent<M: Material + Send + Sync + 'static = StandardMaterial> {
    pub handle: AssetHandle<M>,
}
