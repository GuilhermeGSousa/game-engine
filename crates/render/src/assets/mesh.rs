use essential::assets::{handle::AssetHandle, Asset};

use crate::loaders::obj_loader::ObjLoader;

use super::{material::Material, vertex::Vertex};

pub struct SubMesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub material_index: usize,
}

pub struct Mesh {
    pub meshes: Vec<SubMesh>,
    pub materials: Vec<AssetHandle<Material>>,
}

impl Asset for Mesh {
    type UsageSettings = ();

    fn loader() -> Box<dyn essential::assets::asset_loader::AssetLoader<Asset = Self>> {
        Box::new(ObjLoader)
    }

    fn default_usage_settings() -> Self::UsageSettings {
        ()
    }
}