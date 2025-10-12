use crate::assets::LoadableAsset;

use super::{asset_server::AssetLoadContext, AssetPath};
use async_trait::async_trait;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
pub trait AssetLoader: Send + Sync + 'static {
    type Asset: LoadableAsset + 'static;

    async fn load(
        &self,
        path: AssetPath<'static>,
        load_context: &mut AssetLoadContext,
        usage_setting: <Self::Asset as LoadableAsset>::UsageSettings,
    ) -> Result<Self::Asset, ()>;
}
