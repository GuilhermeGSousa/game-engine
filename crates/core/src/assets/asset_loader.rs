use super::{Asset, AssetId};
use async_trait::async_trait;

#[async_trait]
pub trait AssetLoader: Send + Sync + 'static {
    type Asset: Asset + 'static;

    async fn load(&self, id: AssetId) -> Result<Self::Asset, String> {
        // Placeholder for actual asset loading logic
        // In a real implementation, this would involve file I/O and parsing
        Err("Not implemented".to_string())
    }
}
