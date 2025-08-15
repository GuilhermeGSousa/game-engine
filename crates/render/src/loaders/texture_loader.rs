use essential::assets::{
    asset_loader::AssetLoader, asset_server::AssetLoadContext, utils::load_binary, Asset, AssetPath,
};

use async_trait::async_trait;

use crate::assets::texture::Texture;

pub struct TextureLoader;

#[cfg_attr(target_arch = "wasm32", async_trait(?Send))]
#[cfg_attr(not(target_arch = "wasm32"), async_trait)]
impl AssetLoader for TextureLoader {
    type Asset = Texture;

    async fn load(
        &self,
        path: AssetPath<'static>,
        _load_context: &mut AssetLoadContext,
        usage_settings: <Self::Asset as Asset>::UsageSettings,
    ) -> Result<Self::Asset, ()> {
        let data = load_binary(path).await;
        match data {
            Ok(data) => {
                let texture = Texture::from_bytes(&data, usage_settings);
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
