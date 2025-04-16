use super::{asset_server::AssetLoadContext, Asset, AssetPath};
use async_trait::async_trait;

#[async_trait]
pub trait AssetLoader: Send + Sync + 'static {
    type Asset: Asset + 'static;

    fn gather_dependencies(&self, path: AssetPath) -> Vec<AssetPath> {
        vec![]
    }

    async fn load(&self, path: AssetPath) -> Result<Self::Asset, ()>;
}
