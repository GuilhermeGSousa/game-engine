use ecs::resource::Resource;

use super::{handle::AssetHandle, Asset};

#[derive(Resource)]
pub struct AssetManager {}

impl AssetManager {
    pub fn new() -> Self {
        AssetManager {}
    }

    pub fn load_asset<A: Asset>(&self, path: &str) -> Result<AssetHandle<A>, String> {
        // Placeholder for actual asset loading logic
        // In a real implementation, this would involve file I/O and parsing
        Err("Not implemented".to_string())
    }

    async fn load_asset_async<A: Asset>(&self, path: &str) -> Result<AssetHandle<A>, String> {
        // Placeholder for actual async asset loading logic
        // In a real implementation, this would involve async file I/O and parsing
        Err("Not implemented".to_string())
    }
}
