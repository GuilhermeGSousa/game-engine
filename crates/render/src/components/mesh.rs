use std::sync::Arc;

use ecs::component::Component;

use crate::vertex::Vertex;

pub struct MeshAsset {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
}

impl Drop for MeshAsset {
    fn drop(&mut self) {
        println!("MeshAsset dropped");
    }
}

#[derive(Component)]
pub struct Mesh {
    pub mesh_asset: Arc<MeshAsset>,
}
