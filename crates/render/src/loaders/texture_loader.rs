use essential::assets::{
    asset_loader::AssetLoader, asset_server::AssetLoadContext, utils::load_binary, AssetPath,
};

use crate::assets::texture::Texture;

pub struct TextureLoader;

#[async_trait::async_trait]
impl AssetLoader for TextureLoader {
    type Asset = Texture;

    async fn load(
        &self,
        path: AssetPath,
        _load_context: &mut AssetLoadContext,
    ) -> Result<Self::Asset, ()> {
        let data = load_binary(path).await;
        match data {
            Ok(data) => {
                let texture = Texture::from_bytes(&data);
                match texture {
                    Ok(texture) => return Ok(texture),
                    Err(_) => {
                        return Err(());
                    }
                }
            }
            Err(_) => Err(()),
        }
    }
}
