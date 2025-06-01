use essential::assets::Asset;
use image::GenericImageView;

use crate::loaders::texture_loader::TextureLoader;

pub struct Texture {
    data: Vec<u8>,
    dimensions: (u32, u32),
}

impl Texture {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, ()> {
        let img = image::load_from_memory(bytes).map_err(|_| ())?;

        Ok(Texture {
            data: img.to_rgba8().into_raw(),
            dimensions: img.dimensions(),
        })
    }

    pub fn dimensions(&self) -> (u32, u32) {
        self.dimensions
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

impl Asset for Texture {
    fn loader() -> Box<dyn essential::assets::asset_loader::AssetLoader<Asset = Self>> {
        Box::new(TextureLoader)
    }
}
