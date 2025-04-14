use super::{Asset, AssetPath};
use async_trait::async_trait;

#[async_trait]
pub trait AssetLoader: Send + Sync + 'static {
    type Asset: Asset + 'static;

    async fn load(&self, path: AssetPath) -> Result<Self::Asset, ()>;
}
