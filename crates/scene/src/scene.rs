use anyhow::Context;
use asset_cook::CookedAsset;
use essential::assets::{
    asset_loader::AssetLoader,
    asset_server::{AssetLoadContext, AssetServer},
    handle::AssetHandle,
    Asset, AssetId, AssetPath, LoadableAsset,
};
use essential::transform::Transform;
use mesh::mesh::Mesh;
use render::assets::material::StandardMaterial;
use serde::{Deserialize, Serialize};

/// One node in a [`Scene`]: a named local transform plus optional mesh and
/// material references and the indices of its children within `Scene::nodes`.
///
/// Skeleton/camera/light/extras fields are an explicit follow-up; this type
/// carries the mesh/material/hierarchy core shared by every source format.
#[derive(Serialize, Deserialize)]
pub struct SceneNode {
    pub name: String,
    pub transform: Transform,
    pub children: Vec<usize>,
    pub mesh: Option<AssetHandle<Mesh>>,
    pub material: Option<AssetHandle<StandardMaterial>>,
}

/// A format-agnostic scene graph, serialized directly (no separate DTO) and
/// cooked as its own asset. `nodes[0]` is not special — roots are simply the
/// nodes no other node lists as a child.
#[derive(Asset, Serialize, Deserialize)]
pub struct Scene {
    pub nodes: Vec<SceneNode>,
}

impl CookedAsset for Scene {
    const TYPE_NAME: &'static str = "Scene";

    fn referenced_sub_assets(&self) -> Vec<AssetId> {
        self.nodes
            .iter()
            .flat_map(|node| {
                [
                    node.mesh.as_ref().map(|handle| handle.id()),
                    node.material.as_ref().map(|handle| handle.id()),
                ]
            })
            .flatten()
            .collect()
    }
}

impl Scene {
    /// Upgrades every `Weak` mesh/material handle to a live `Strong` one via
    /// the given `AssetServer`, kicking off their loads.
    pub(crate) fn resolve_asset_handles(&mut self, asset_server: &AssetServer) {
        for node in &mut self.nodes {
            if let Some(handle) = &mut node.mesh {
                *handle = asset_server.load_by_id(handle.id());
            }
            if let Some(handle) = &mut node.material {
                *handle = asset_server.load_by_id(handle.id());
            }
        }
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
        _path: AssetPath<'static>,
        load_context: &mut AssetLoadContext,
        _usage_settings: (),
    ) -> anyhow::Result<Self::Asset> {
        let bytes = essential::assets::utils::load_cooked_asset_bytes(
            load_context.cooked_root(),
            load_context.asset_id(),
        )
        .await
        .with_context(|| "failed to read cooked scene")?;
        let mut scene: Scene =
            bincode::deserialize(&bytes).with_context(|| "failed to deserialize cooked scene")?;
        scene.resolve_asset_handles(load_context.asset_server());
        Ok(scene)
    }
}
