use essential::assets::{handle::AssetHandle, Asset};

use super::texture::Texture;

pub struct Material {
    diffuse_texture: Option<AssetHandle<Texture>>,
}

impl Material {}

impl Asset for Material {
    fn loader() -> Box<dyn essential::assets::asset_loader::AssetLoader<Asset = Self>> {
        todo!()
    }
}
