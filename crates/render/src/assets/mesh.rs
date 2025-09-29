use essential::assets::{Asset, LoadableAsset};

use crate::loaders::obj_loader::ObjLoader;

use super::vertex::Vertex;

pub struct Mesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl Asset for Mesh {}

impl LoadableAsset for Mesh {
    type UsageSettings = ();

    fn loader() -> Box<dyn essential::assets::asset_loader::AssetLoader<Asset = Self>> {
        Box::new(ObjLoader)
    }

    fn default_usage_settings() -> Self::UsageSettings {
        ()
    }
}
