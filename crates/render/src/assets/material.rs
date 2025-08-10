use essential::assets::{handle::AssetHandle, Asset};

use crate::loaders::mtl_loader::MTLLoader;

use super::texture::Texture;

pub struct Material {
    diffuse_texture: Option<AssetHandle<Texture>>,
    normal_texture: Option<AssetHandle<Texture>>,
}

impl Material {
    pub fn new() -> Self {
        Self {
            diffuse_texture: None,
            normal_texture: None,
        }
    }

    pub fn set_diffuse_texture(&mut self, texture: AssetHandle<Texture>) {
        self.diffuse_texture = Some(texture);
    }

    pub fn diffuse_texture(&self) -> Option<&AssetHandle<Texture>> {
        self.diffuse_texture.as_ref()
    }

    pub fn set_normal_texture(&mut self, texture: AssetHandle<Texture>) {
        self.normal_texture = Some(texture);
    }

    pub fn normal_texture(&self) -> Option<&AssetHandle<Texture>> {
        self.normal_texture.as_ref()
    }
}

impl Asset for Material {
    fn loader() -> Box<dyn essential::assets::asset_loader::AssetLoader<Asset = Self>> {
        Box::new(MTLLoader)
    }
}
