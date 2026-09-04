use anyhow::Context;
use async_trait::async_trait;
use ecs::{Component, Entity};
use essential::assets::{
    asset_loader::AssetLoader, asset_server::AssetLoadContext, handle::AssetHandle, Asset,
    AssetPath, LoadableAsset,
};
use glam::Mat4;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Asset, Serialize, Deserialize)]
pub struct Skeleton {
    pub inverse_bindposes: Box<[Mat4]>,
}

impl From<Vec<Mat4>> for Skeleton {
    fn from(value: Vec<Mat4>) -> Self {
        Self {
            inverse_bindposes: value.into_boxed_slice(),
        }
    }
}

impl LoadableAsset for Skeleton {
    type UsageSettings = ();
    fn loader() -> Box<dyn AssetLoader<Asset = Self>> {
        Box::new(SkeletonLoader)
    }
    fn default_usage_settings() -> Self::UsageSettings {}
}

pub struct SkeletonLoader;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl AssetLoader for SkeletonLoader {
    type Asset = Skeleton;

    async fn load(
        &self,
        path: AssetPath<'static>,
        load_context: &mut AssetLoadContext,
        _usage_settings: (),
    ) -> anyhow::Result<Self::Asset> {
        let bytes = essential::assets::utils::load_content_asset_bytes(
            load_context.content_root(),
            &path.address(),
            Skeleton::name(),
        )
        .await
        .with_context(|| "failed to read skeleton asset")?;
        bincode::deserialize(&bytes).with_context(|| "failed to deserialize skeleton asset")
    }
}

#[derive(Component, Clone)]
pub struct SkeletonComponent {
    skeleton: AssetHandle<Skeleton>,
    bones: Vec<Entity>,
    bone_ids: Vec<Uuid>,
}

impl SkeletonComponent {
    pub fn new(skeleton: AssetHandle<Skeleton>, bones: Vec<Entity>, bone_ids: Vec<Uuid>) -> Self {
        Self {
            skeleton,
            bones,
            bone_ids,
        }
    }

    pub fn skeleton(&self) -> &AssetHandle<Skeleton> {
        &self.skeleton
    }

    pub fn bones(&self) -> &[Entity] {
        &self.bones
    }

    pub fn bone_ids(&self) -> &[Uuid] {
        &self.bone_ids
    }
}
