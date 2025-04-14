use essential::assets::{handle::AssetHandle, Asset};
use std::sync::Arc;

use ecs::component::Component;
use vertex::Vertex;

pub mod obj_loader;
pub mod render_mesh;
pub mod vertex;

pub struct MeshAsset {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
}

impl Drop for MeshAsset {
    fn drop(&mut self) {
        println!("MeshAsset dropped");
    }
}

pub struct ModelAsset {
    pub meshes: Vec<MeshAsset>,
}

impl Asset for ModelAsset {
    fn loader() -> Box<dyn essential::assets::asset_loader::AssetLoader<Asset = Self>> {
        Box::new(obj_loader::ObjLoader)
    }
}

#[derive(Component)]
pub struct Mesh {
    pub mesh_asset: Arc<MeshAsset>,
    pub handle: AssetHandle<ModelAsset>,
}
