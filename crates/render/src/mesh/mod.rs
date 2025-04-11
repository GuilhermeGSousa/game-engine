use core::assets::Asset;
use std::sync::Arc;

use ecs::component::Component;
use vertex::Vertex;

pub mod obj_loader;
pub mod render_mesh;
pub mod vertex;

pub struct MeshAsset {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
}

impl Drop for MeshAsset {
    fn drop(&mut self) {
        println!("MeshAsset dropped");
    }
}

impl Asset for MeshAsset {
    fn loader() -> Box<dyn core::assets::asset_loader::AssetLoader<Asset = Self>> {
        todo!()
    }

    fn id(path: String) -> core::assets::AssetId {
        todo!()
    }
}

#[derive(Component)]
pub struct Mesh {
    pub mesh_asset: Arc<MeshAsset>,
}
