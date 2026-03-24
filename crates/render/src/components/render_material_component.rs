use std::marker::PhantomData;

use ecs::component::Component;
use essential::assets::AssetId;

/// Render-world component placed on mesh entities to identify which material
/// asset they use for a specific material type `M`.
///
/// This component replaces the old `MaterialInstanceTag<M>` (which was a pure
/// phantom marker) by also carrying the material's [`AssetId`].  This means
/// [`super::mesh_component::RenderMeshInstance`] no longer needs to store the
/// material asset id — the two concerns (mesh geometry and material) are kept
/// in separate components.
///
/// The type parameter `M` ensures that `material_renderpass<M>` only picks up
/// entities belonging to pipeline `M`, so multiple `MaterialPlugin` instances
/// for different material types coexist without interfering with each other.
pub(crate) struct RenderMaterialComponent<M: 'static> {
    pub(crate) material_asset_id: AssetId,
    _marker: PhantomData<fn() -> M>,
}

impl<M: 'static> RenderMaterialComponent<M> {
    pub(crate) fn new(material_asset_id: AssetId) -> Self {
        Self {
            material_asset_id,
            _marker: PhantomData,
        }
    }
}

impl<M: Send + Sync + 'static> Component for RenderMaterialComponent<M> {
    fn name() -> &'static str {
        std::any::type_name::<RenderMaterialComponent<M>>()
    }
}
