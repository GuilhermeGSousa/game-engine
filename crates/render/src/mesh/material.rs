use essential::assets::{handle::AssetHandle, Asset};

use super::texture::Texture;

pub struct Material {
    diffuse_texture: Option<AssetHandle<Texture>>,
}

impl Material {
    pub fn new() -> Self {
        Self {
            diffuse_texture: None,
        }
    }

    pub fn set_diffuse_texture(&mut self, texture: AssetHandle<Texture>) {
        self.diffuse_texture = Some(texture);
    }
}

impl Asset for Material {
    fn loader() -> Box<dyn essential::assets::asset_loader::AssetLoader<Asset = Self>> {
        todo!()
    }
}
