use anyhow::Context;
use ecs::component::Component;
use essential::assets::{
    asset_loader::AssetLoader, asset_server::AssetLoadContext, Asset, AssetId, AssetPath,
    LoadableAsset,
};
use serde::{Deserialize, Serialize};

/// One component's serialized payload: the registry key it was registered under
/// plus its serde-JSON encoding.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializedComponent {
    pub type_name: String,
    pub data: String,
}

/// One node in a [`Scene`]: a name, the indices of its children within
/// `Scene::nodes`, and the list of components authored onto it. Every
/// runtime concern (transform, mesh, material, camera, light, ...) is carried
/// as a [`SerializedComponent`]; the spawner applies them generically through
/// the component registry.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SceneNode {
    pub name: String,
    /// Indices into `Scene::nodes`.
    pub children: Vec<usize>,
    pub components: Vec<SerializedComponent>,
}

impl SceneNode {
    /// Serializes `component` and appends it to this node. Importers use this
    /// rather than building [`SerializedComponent`] by hand, so the registry
    /// key always comes from [`Component::name`].
    pub fn push_component<T: Serialize + Component>(
        &mut self,
        component: &T,
    ) -> anyhow::Result<()> {
        self.components.push(SerializedComponent {
            type_name: T::name().to_string(),
            data: serde_json::to_string(component)?,
        });
        Ok(())
    }
}

/// A format-agnostic scene graph, serialized directly (no separate DTO) and
/// serialized as its own asset. `nodes[0]` is not special — roots are simply the
/// nodes no other node lists as a child.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Scene {
    pub nodes: Vec<SceneNode>,
    /// Every [`AssetId`] reachable from the nodes' components. Component
    /// payloads are opaque strings, so [`Asset::referenced_sub_assets`]
    /// cannot introspect them — the importer records the ids here as it emits.
    pub referenced_assets: Vec<AssetId>,
}

impl Asset for Scene {
    fn name() -> &'static str {
        "Scene"
    }

    fn referenced_sub_assets(&self) -> Vec<AssetId> {
        self.referenced_assets.clone()
    }
}

impl LoadableAsset for Scene {
    type UsageSettings = ();

    fn loader() -> Box<dyn AssetLoader<Asset = Self>> {
        Box::new(SceneLoader)
    }

    fn default_usage_settings() -> Self::UsageSettings {}
}

pub struct SceneLoader;

#[cfg_attr(target_arch = "wasm32", async_trait::async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait::async_trait)]
impl AssetLoader for SceneLoader {
    type Asset = Scene;

    async fn load(
        &self,
        path: AssetPath<'static>,
        load_context: &mut AssetLoadContext,
        _usage_settings: (),
    ) -> anyhow::Result<Self::Asset> {
        let bytes = essential::assets::utils::load_content_asset_bytes(
            load_context.content_root(),
            &path.address(),
            Scene::name(),
        )
        .await
        .with_context(|| "failed to read scene asset")?;
        // Each component upgrades its own Weak handle in `apply`, so the
        // loader no longer resolves anything itself.
        bincode::deserialize(&bytes).with_context(|| "failed to deserialize scene asset")
    }
}
