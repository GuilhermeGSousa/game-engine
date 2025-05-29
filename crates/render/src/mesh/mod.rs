use essential::assets::{handle::AssetHandle, Asset};
use material::Material;

use ecs::component::Component;
use vertex::Vertex;

pub mod layouts;
pub mod material;
pub mod obj_loader;
pub mod render_instanced_mesh;
pub mod render_material;
pub mod render_mesh;
pub mod render_texture;
pub mod texture;
pub mod texture_loader;
pub mod vertex;

pub struct SubMesh {
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u32>,
    pub material_index: usize,
}

impl Drop for SubMesh {
    fn drop(&mut self) {
        println!("MeshAsset dropped");
    }
}

pub struct Mesh {
    pub meshes: Vec<SubMesh>,
    pub materials: Vec<AssetHandle<Material>>,
}

impl Asset for Mesh {
    fn loader() -> Box<dyn essential::assets::asset_loader::AssetLoader<Asset = Self>> {
        Box::new(obj_loader::ObjLoader)
    }
}

#[derive(Component)]
pub struct MeshComponent {
    pub handle: AssetHandle<Mesh>,
}
