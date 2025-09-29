use essential::assets::{Asset, LoadableAsset};

use crate::loaders::gltf_loader::GLTFLoader;

pub struct Scene {}

impl Asset for Scene {}

impl LoadableAsset for Scene {
    type UsageSettings = ();

    fn loader() -> Box<dyn essential::assets::asset_loader::AssetLoader<Asset = Self>> {
        Box::new(GLTFLoader)
    }

    fn default_usage_settings() -> Self::UsageSettings {
        ()
    }
}
